//! The running game's power tree screen and power bar: the original's `P`
//! key opens the tree, `C` the bar; the pointer hovers, clicks, drags and
//! zooms with the tree's rules and the canvas shows the original's picture.
//!
//! The screen's state is [`TreeScreen`] (the rules), [`PowerBar`] and the
//! character's [`PowerRecord`]. The draw is built by
//! [`TreeScreen::view`] and [`exa_ui::tree_draw`] and rasterised here with
//! the six fragment programs' arithmetic and the loaded `.rfi` sheets, into
//! one screen-sized image (the same canvas path the main menu uses). Only
//! the open screen is drawn, at most 33 ms apart; the in-game bar draws a
//! strip `scale × 20` high.
//!
//! The text is the GUI skin's own: the interface level's main font gives
//! the advances, glyph quad and line step, and its glyphs sample the level's
//! atlas when the install ships it, else the base font's atlas scaled to
//! that quad (only `fontbase24r` ships). The tooltip is the shown choice's
//! name caption (at once) and its wrapped description box (after 600 ms),
//! laid out at the original's places; while a power is dragged every node
//! wears its name instead. The domain wheel child draws its own paint
//! ([`exa_ui::wheel_draw`]) and is hovered every frame. While the tree is shown its
//! form has the keys: the arrows step the bar and any other key closes it
//! ([`TreeScreen::key`]). The bar's own keys (pick, step, use) belong to
//! the hotkey handler and the cast plugin ([`crate::thaum::cast`]), which
//! keeps the caster's record and this screen's in step.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{ComputeTaskPool, TaskPool};
use bevy::window::PrimaryWindow;
use exa_assets::{Game, RgbaImage, TreeSheets};
use exa_audio::{AudioEnvironment, SoundBank};
use exa_formats::rfi::Rfi;
use exa_ui::font::{BASE_FONT, GameFont};
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask};
use exa_ui::gui_font::{FONTBASE_24, GuiFont, font_with_fallback, gui_level, level_for_unit};
use exa_ui::tree_draw::{self, Batch, Program, TextRun, Texture, TreeDraw, TreeVertex};

use crate::bindings::{Action, Bindings};
use crate::thaum::tree::{BAR_CLOSING, PowerBar, PowerRecord, TreeMode, TreeScreen, TreeSound};

/// The time between redraws while the screen animates.
const REDRAW_MS: u128 = 33;
/// The bar's canvas height in GUI units.
const BAR_UNITS: f32 = 20.0;
/// The canvas's layer: over the HUD's (the heartbeat's border at 890, the
/// death fade at 895, the compass, status and bars up to 900), as the
/// original draws its GUI forms after the HUD; under the splash, the level
/// transition and the cursor.
pub const TREE_Z: i32 = 910;

/// The screen's state.
#[derive(Resource)]
pub struct PowerScreen {
    pub open: bool,
    pub bar_open: bool,
    pub tree: TreeScreen,
    pub record: PowerRecord,
    pub bar: PowerBar,
    pub texts: Vec<Vec<u8>>,
    pub domain_files: [Option<Vec<u8>>; 7],
    pub sheets: TreeSheets,
    /// The GUI font atlas the tree's text samples and its texture name.
    pub font_atlas: Option<(&'static str, (u16, u16))>,
    pub font: Option<GameFont>,
    /// The sound the tree's last action names, for the audio path: showing
    /// it plays `ThaumShow02`; a hidden screen is silent. The running game
    /// has no UI-sound playback yet.
    pub sound: Option<TreeSound>,
    /// A redraw is needed.
    pub dirty: bool,
    /// The canvas kept between redraws.
    pub canvas: CanvasCache,
    /// The frames sheet at mip level 4, which the frame program also reads.
    pub frames_coarse: Option<RgbaImage>,
    /// The last draw, in the frame clock's milliseconds.
    last_draw: u128,
    /// The dragged power's preview.
    drag: Option<try_drag::Dragged>,
    /// Where the left button went down, until it has dragged or let go.
    press_origin: Option<[f32; 2]>,
}

mod try_drag {
    /// A power being dragged, with where the pointer is.
    #[derive(Clone)]
    pub struct Dragged {
        pub power: u32,
        pub flags: u32,
        pub extra: u32,
        pub origin: usize,
        pub at: [f32; 2],
    }
}

impl Default for PowerScreen {
    fn default() -> Self {
        Self {
            open: false,
            bar_open: false,
            tree: TreeScreen::empty(1, TreeMode::Play),
            record: PowerRecord::starter(),
            bar: PowerBar::default(),
            texts: Vec::new(),
            domain_files: [None, None, None, None, None, None, None],
            sheets: TreeSheets::default(),
            font_atlas: None,
            font: None,
            sound: None,
            dirty: true,
            canvas: CanvasCache::default(),
            frames_coarse: None,
            last_draw: 0,
            drag: None,
            press_origin: None,
        }
    }
}

/// The loaded assets are in.
#[derive(Resource)]
struct AssetsLoaded;

pub struct PowerScreenPlugin;

impl Plugin for PowerScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PowerScreen>()
            .add_systems(Startup, (load_screen, setup_router))
            .add_systems(Update, (hotkeys, pointer, draw, ui_sounds).chain());
    }
}

/// The tree's form in the GUI core's task, so the pointer takes the
/// window manager's route.
#[derive(Resource)]
struct PowerRouter {
    task: GuiTask,
    form: FormId,
    state: Arc<Mutex<PowerFormState>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct PowerFormState {
    open: bool,
    width: i32,
    height: i32,
}

struct PowerForm {
    state: Arc<Mutex<PowerFormState>>,
}

impl FormHost for PowerForm {
    fn parent(&self) -> Option<FormId> {
        None
    }

    fn bounds(&self) -> GuiRect {
        let state = self.state.lock().map(|s| *s).unwrap_or_default();
        GuiRect::new(0, 0, state.width, state.height)
    }

    fn is_visible(&self) -> bool {
        self.state.lock().map(|state| state.open).unwrap_or(false)
    }

    fn hit_test(&self, x: i32, y: i32) -> i32 {
        let state = self.state.lock().map(|s| *s).unwrap_or_default();
        if state.open && (0..state.width).contains(&x) && (0..state.height).contains(&y) {
            1
        } else {
            -1
        }
    }
}

fn setup_router(mut commands: Commands) {
    let state = Arc::new(Mutex::new(PowerFormState::default()));
    let mut task = GuiTask::new((800, 600));
    let form = task.add(Box::new(PowerForm {
        state: Arc::clone(&state),
    }));
    commands.insert_resource(PowerRouter { task, form, state });
}

fn load_screen(
    mut commands: Commands,
    game: Option<Res<Game>>,
    gui: Option<Res<exa_ui::GuiScale>>,
    mut screen: ResMut<PowerScreen>,
) {
    if let Some(game) = game.as_deref() {
        const FILES: [&str; 7] = [
            "",
            "pwr_mind.pwr",
            "pwr_force.pwr",
            "pwr_body.pwr",
            "pwr_energy.pwr",
            "pwr_light.pwr",
            "pwr_displace.pwr",
        ];
        for (index, name) in FILES.iter().enumerate().skip(1) {
            if let Some(bytes) = game.0.resource_file(name) {
                screen.domain_files[index] = Some(bytes.to_vec());
            }
        }
        screen.sheets = game.load_tree_sheets();
        screen.frames_coarse = screen.sheets.frames.as_ref().map(level_four);
        screen.canvas.invalidate();
        load_tree_font(
            game,
            gui.as_deref().map(|gui| gui.scale as i32),
            &mut screen,
        );
        if let Some(bytes) = game.0.texture(BASE_FONT)
            && let Ok(rfi) = Rfi::parse(bytes)
        {
            screen.font = GameFont::from_rfi(&rfi);
        }
        let file = screen.domain_files.get(1).cloned().flatten();
        screen.tree = TreeScreen::load(
            1,
            TreeMode::Play,
            Some(&screen.record),
            file.as_deref(),
            1000.0,
            700.0,
        );
        screen.tree.domain_cells = domain_cells(&screen.sheets);
        screen.texts = Vec::new();
    }
    commands.insert_resource(AssetsLoaded);
}

/// Loads the tree's text atlas: the interface level's main font when the
/// install ships it, else the base font's atlas that the packs carry. The
/// level's metrics stay either way (see [`font_with_fallback`]).
fn load_tree_font(game: &Game, unit: Option<i32>, screen: &mut PowerScreen) {
    let level = gui_level(level_for_unit(unit.unwrap_or(4))).main;
    let loaded = game
        .load_rgba(level.texture)
        .ok()
        .map(|image| (image, level.texture))
        .or_else(|| {
            game.load_rgba(FONTBASE_24.texture)
                .ok()
                .map(|image| (image, FONTBASE_24.texture))
        });
    if let Some((image, name)) = loaded {
        let size = (image.width as u16, image.height as u16);
        screen.sheets.font = Some(image);
        screen.font_atlas = Some((name, size));
    } else {
        screen.sheets.font = None;
        screen.font_atlas = None;
    }
}

/// The tree's font at the interface unit `unit` and the atlas size its
/// glyph quads sample, from the loaded atlas (the draw sets the tree's
/// font size from it every frame). Another level's atlas gives no glyphs.
pub fn tree_font(unit: i32, atlas: Option<(&'static str, (u16, u16))>) -> (GuiFont, (u16, u16)) {
    let level = gui_level(level_for_unit(unit)).main;
    let level_size = atlas
        .filter(|(name, _)| *name == level.texture)
        .map(|(_, size)| size);
    let base_size = atlas
        .filter(|(name, _)| *name == FONTBASE_24.texture)
        .map(|(_, size)| size);
    let (font, size) = font_with_fallback(level, level_size, base_size);
    (font, size.unwrap_or((0, 0)))
}

/// Each installed domain sheet's cells per side: its width over the
/// original's 512-pixel cell. A missing sheet stays zero, so its icons are
/// not drawn (the shipped sheets are 2048 or 512 pixels square).
pub fn domain_cells(sheets: &TreeSheets) -> [u32; 7] {
    let mut cells = [0u32; 7];
    for (index, sheet) in sheets.domains.iter().enumerate().skip(1) {
        if let Some(sheet) = sheet {
            cells[index] = sheet.width >> 9;
        }
    }
    cells
}

#[allow(clippy::too_many_arguments)]
fn hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Option<Res<Bindings>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut screen: ResMut<PowerScreen>,
) {
    let Some(bindings) = bindings else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as f32;
    let height = window.physical_height() as f32;
    let now = time.elapsed().as_millis() as u32;
    let PowerScreen {
        open,
        bar_open,
        tree,
        record,
        bar,
        texts,
        domain_files,
        dirty,
        sound,
        ..
    } = &mut *screen;
    // While the tree is shown its form has the keys: every key goes to the
    // tree's key slot, which steps the bar on the arrows and closes the
    // tree on any other key (after the first 50 ms of the opening fade, and
    // not again while it fades out).
    if *open {
        let pressed: Vec<u32> = keys
            .get_just_pressed()
            .filter_map(|&key| crate::bindings::virtual_key(key))
            .collect();
        for key in pressed {
            let out = tree.key(
                crate::thaum::tree::KeyInput {
                    key: key as i32,
                    flags: 0,
                    guarded: tree.fade_flags & crate::thaum::tree::FADING_IN != 0,
                    hidden: tree.closing(),
                    stamp: tree.open_at,
                    now,
                },
                bar,
                Some(record),
            );
            if out.sound.is_some() {
                *sound = out.sound;
            }
            if out.toggle {
                // Closing stores the bar and the domain, then fades out
                // over 256 ms; the draw turns the screen off at the fade's
                // end.
                tree.leave(bar, record);
                tree.hide(now);
                tree.close_bar(now);
            }
            if out.toggle || out.cycled {
                *dirty = true;
            }
        }
        return;
    }
    if bindings
        .key_code(Action::PowerTree)
        .is_some_and(|key| keys.just_pressed(key))
    {
        // A record that knows no power starts with the two default pairs,
        // as the tree's first opening seeds them.
        crate::thaum::tree::seed_defaults(record);
        let domain = tree.domain;
        let file = domain_files.get(usize::from(domain)).cloned().flatten();
        *texts = file
            .as_deref()
            .and_then(|bytes| exa_formats::power_tree::read_power_tree(bytes).ok())
            .map(|tree| tree.texts)
            .unwrap_or_default();
        *tree = TreeScreen::load(
            domain,
            TreeMode::Play,
            Some(&*record),
            file.as_deref(),
            width,
            height,
        );
        *open = true;
        *bar_open = true;
        tree.bar_open = true;
        *sound = tree.show(now).sound;
        tree.open_bar_if_closed(bar, record, now);
        *dirty = true;
        return;
    }
    // Escape with the tree closed puts the bar away.
    if keys.just_pressed(KeyCode::Escape) {
        *bar_open = false;
        *dirty = true;
    }
}

/// How far the pointer goes with the left button down before a drag starts.
const DRAG_DISTANCE: f32 = 6.0;

/// The pointer's tree input.
#[allow(clippy::too_many_arguments)]
fn pointer(
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut screen: ResMut<PowerScreen>,
    mut router: Option<ResMut<PowerRouter>>,
) {
    if !screen.open {
        while wheel.read().next().is_some() {}
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as f32;
    let height = window.physical_height() as f32;
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let x = cursor.x * window.scale_factor();
    let y = cursor.y * window.scale_factor();
    let now = time.elapsed().as_millis() as u32;
    if let Some(router) = router.as_deref_mut() {
        router.task.screen = (width as i32, height as i32);
        if let Ok(mut state) = router.state.lock() {
            state.open = screen.open;
            state.width = width as i32;
            state.height = height as i32;
        }
        let hit = router.task.hit_and_focus(x as i32, y as i32);
        if hit.form != Some(router.form) {
            return;
        }
    }
    let PowerScreen {
        open,
        tree,
        record,
        bar,
        dirty,
        drag,
        press_origin,
        sound,
        ..
    } = &mut *screen;
    let hidden = !*open || tree.closing();
    // The domain wheel is a child form, so a button over one of its icons
    // is its own: the click (the release) confirms that domain through the
    // wheel's rule and turns the tree to it.
    if !hidden && drag.is_none() && tree.wheel_child.domain_at(x, y) > 0 {
        if buttons.just_released(MouseButton::Left) {
            *press_origin = None;
            if let Some(played) = tree.wheel_click(x, y, now) {
                *sound = Some(played);
            }
            *dirty = true;
            return;
        }
        if buttons.just_pressed(MouseButton::Left) {
            return;
        }
    }
    // The wheel child's own hover: the icon under the pointer, its
    // highlight stamp and the label it keeps.
    if !hidden {
        let hit = tree.wheel_child.domain_at(x, y);
        tree.wheel_child.state.hover(i32::from(hit), now);
    }
    let hover = tree.hover(x, y, width, height, hidden);
    if hover != tree.focus {
        tree.set_focus(hover, now);
        *dirty = true;
    }
    if let Some(held) = drag.as_mut() {
        held.at = [x, y];
        *dirty = true;
    }
    if buttons.just_pressed(MouseButton::Left) {
        let input = crate::thaum::tree::PressInput {
            x,
            y,
            word: crate::thaum::tree::press::LEFT | crate::thaum::tree::press::PRESS,
            width,
            now,
            hidden,
        };
        *dirty = true;
        tree.drag = crate::thaum::tree::DragState::Idle;
        let out = tree.press(input, Some(record), bar);
        if out.sound.is_some() {
            *sound = out.sound;
        }
        *press_origin = Some([x, y]);
    }
    if buttons.pressed(MouseButton::Left)
        && let Some(origin) = *press_origin
        && drag.is_none()
        && (x - origin[0]).hypot(y - origin[1]) >= DRAG_DISTANCE
    {
        // A learnt node or a bar slot starts a power drag once the pointer
        // has moved; a press that stays is a click.
        *press_origin = None;
        let drag_input = crate::thaum::tree::PressInput {
            x: origin[0],
            y: origin[1],
            word: crate::thaum::tree::press::LEFT | crate::thaum::tree::press::DRAG,
            width,
            now,
            hidden,
        };
        if let Some(started) = tree.press(drag_input, Some(record), bar).drag {
            *drag = Some(try_drag::Dragged {
                power: started.power,
                flags: started.flags,
                extra: started.extra,
                origin: started.origin,
                at: [x, y],
            });
        }
    }
    if buttons.just_released(MouseButton::Left) {
        *press_origin = None;
        let input = crate::thaum::tree::PressInput {
            x,
            y,
            word: crate::thaum::tree::press::LEFT | crate::thaum::tree::press::CLICK,
            width,
            now,
            hidden,
        };
        if let Some(held) = drag.take() {
            // A drag ends on the bar: the tree's drop rule fills a slot.
            let power = crate::thaum::tree::PowerDrag {
                power: held.power,
                flags: held.flags,
                extra: held.extra,
                kind: crate::thaum::tree::press::POWER_DRAG,
                origin: held.origin,
            };
            let (_, drop_sound) = bar.drop(x, y, width, &power);
            if drop_sound.is_some() {
                *sound = drop_sound;
            }
        } else {
            let out = tree.press(input, Some(record), bar);
            if out.sound.is_some() {
                *sound = out.sound;
            }
        }
        *dirty = true;
    }
    if buttons.just_pressed(MouseButton::Middle) {
        // The middle button starts a pan from the view as shown.
        let input = crate::thaum::tree::PressInput {
            x,
            y,
            word: crate::thaum::tree::press::MIDDLE,
            width,
            now,
            hidden,
        };
        let _ = tree.press(input, Some(record), bar);
        *dirty = true;
    }
    if buttons.pressed(MouseButton::Middle) && tree.drag == crate::thaum::tree::DragState::Pan {
        tree.drag_move(x, y, now);
        *dirty = true;
    }
    if buttons.just_released(MouseButton::Middle) && tree.drag == crate::thaum::tree::DragState::Pan
    {
        tree.release_drag(x, y);
        *dirty = true;
    }
    for event in wheel.read() {
        let out = tree.wheel(0x20, 0.0, event.y, now);
        if out.result == 3 {
            *dirty = true;
        }
    }
}

/// The canvases: the tree's full window and the bar's strip.
#[derive(Component)]
pub struct PowerCanvas;

#[derive(Component, Default)]
struct CanvasState {
    size: (u32, u32),
    open: bool,
}

/// Draws the open screen into its image.
#[allow(clippy::too_many_arguments)]
fn draw(
    time: Res<Time>,
    mut screen: ResMut<PowerScreen>,
    gui: Option<Res<exa_ui::GuiScale>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut canvas: Query<
        (&mut ImageNode, &mut Node, &mut Visibility, &mut CanvasState),
        With<PowerCanvas>,
    >,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width();
    let height = window.physical_height();
    if width == 0 || height == 0 {
        return;
    }
    let animating = screen.tree.bar_flags != 0;
    if !screen.open && !screen.bar_open && !animating {
        for (_, _, mut visibility, _) in &mut canvas {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let now = time.elapsed().as_millis();
    if screen.open || screen.bar_open || animating {
        let now_at = now as u32;
        // The bare bar (the tree closed) starts its rise when it is turned
        // on and falls when it is turned off.
        if !screen.open {
            if screen.bar_open && screen.tree.bar_flags == 0 {
                let PowerScreen {
                    tree, bar, record, ..
                } = &mut *screen;
                tree.open_bar(bar, record, now_at);
            } else if !screen.bar_open && screen.tree.bar_flags & BAR_CLOSING == 0 {
                screen.tree.close_bar(now_at);
            }
        }
        screen.tree.present_window(now_at);
        if screen.tree.switch_due {
            // The old tree has faded out: store it, load the chosen
            // domain's tree and start the new fade-in.
            let domain = screen.tree.pending_domain;
            let PowerScreen {
                tree,
                record,
                domain_files,
                texts,
                ..
            } = &mut *screen;
            let file = domain_files.get(usize::from(domain)).cloned().flatten();
            *texts = file
                .as_deref()
                .and_then(|bytes| exa_formats::power_tree::read_power_tree(bytes).ok())
                .map(|tree| tree.texts)
                .unwrap_or_default();
            let font_size = tree.font_size;
            tree.switch_domain(domain, record, file.as_deref(), width as f32, height as f32);
            tree.font_size = font_size;
            screen.dirty = true;
        }
        let cursor = window
            .cursor_position()
            .map(|c| [c.x * window.scale_factor(), c.y * window.scale_factor()]);
        screen.tree.advance_at(now_at, cursor);
        let PowerScreen { tree, bar, .. } = &mut *screen;
        tree.step_bar(bar, now_at);
        if screen.tree.faded_out {
            // The form and its bar child hide together; the bar's own
            // closing animation is for the bar on its own.
            screen.open = false;
            screen.bar_open = false;
            screen.tree.bar_flags = 0;
            screen.dirty = true;
        }
    }
    if !screen.open && !screen.bar_open && screen.tree.bar_flags == 0 {
        for (_, _, mut visibility, _) in &mut canvas {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    if !screen.dirty && now.saturating_sub(screen.last_draw) < REDRAW_MS {
        return;
    }
    // The interface unit is the original's GUI scale, picked from the
    // window's physical height; the tree's layout and its font level both
    // come from it.
    let scale = gui.as_deref().map_or(4, |gui| gui.scale) as f32;
    let (draw, canvas_height) = frame_draw(&mut screen, width, height, scale, now);
    let PowerScreen {
        canvas: cache,
        sheets,
        frames_coarse,
        font,
        open,
        bar_open,
        ..
    } = &mut *screen;
    let size = (width, canvas_height);
    let redrawn = cache.update(draw, sheets, frames_coarse.as_ref(), font.as_ref(), size);
    let canvas_image = |pixels: &[u8]| {
        Image::new(
            Extent3d {
                width,
                height: canvas_height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels.to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        )
    };
    let ui_scale = window.scale_factor();
    let mut spawned = false;
    for (mut image_node, mut node, mut visibility, mut state) in &mut canvas {
        // Only a redrawn canvas is uploaded again, into the same image
        // while its size holds.
        let kept = state.size == size && images.contains(&image_node.image);
        if !kept {
            image_node.image = images.add(canvas_image(cache.straight()));
        } else if !redrawn.is_empty()
            && let Some(mut image) = images.get_mut(&image_node.image)
        {
            image.data = Some(cache.straight().to_vec());
        }
        node.width = Val::Px(width as f32 / ui_scale);
        node.height = Val::Px(canvas_height as f32 / ui_scale);
        node.left = Val::Px(0.0);
        node.top = Val::Px(0.0);
        *visibility = Visibility::Inherited;
        state.size = size;
        state.open = *open || *bar_open;
        spawned = true;
    }
    if !spawned {
        commands.spawn((
            PowerCanvas,
            ImageNode::new(images.add(canvas_image(cache.straight()))),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(width as f32 / ui_scale),
                height: Val::Px(canvas_height as f32 / ui_scale),
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..Node::default()
            },
            GlobalZIndex(TREE_Z),
            CanvasState { size, open: true },
        ));
    }
    screen.last_draw = now;
    screen.dirty = false;
}

/// The open screen's draw for one frame at the interface unit `scale`:
/// the tree, wheel, prompt, tooltip, labels and preview while the tree is
/// shown, else the bare bar; with the canvas height it fills.
pub fn frame_draw(
    screen: &mut PowerScreen,
    width: u32,
    height: u32,
    scale: f32,
    now: u128,
) -> (TreeDraw, u32) {
    let (font, font_size) = tree_font(scale as i32, screen.font_atlas);
    screen.tree.font_size = font_size;
    screen.tree.domain_cells = domain_cells(&screen.sheets);
    // The wheel's texture follows the interface unit (a no-op unless it
    // changed).
    screen.sheets.resample_wheel(scale as i32);
    let tree_open = screen.open;
    let PowerScreen {
        tree,
        bar,
        texts,
        drag,
        ..
    } = &mut *screen;
    let canvas_height = if tree_open {
        height
    } else {
        (scale * BAR_UNITS) as u32
    };
    tree.labels = drag.is_some();
    let view = tree.view(bar, width as f32, height as f32, scale, font, now as u32);
    let draw = if tree_open {
        // The tree pass already ends with the bar.
        let mut draw = tree_draw::tree_draw(&view);
        // The domain wheel child at the form's lower left: its own paint
        // (backdrop, sector bands, choice glow, icons, ring frame, label).
        let wheel_view = tree.wheel_draw_view(now as u32, scale as i32, font, font_size);
        let wheel = exa_ui::wheel_draw::wheel_draw(&wheel_view);
        draw.batches.extend(wheel.batches);
        draw.texts.extend(wheel.texts);
        // The tooltip: the shown choice's name caption and its wrapped
        // description box, measured with the GUI font's advances (the
        // glyphs are the running font's).
        if view.mode == tree_draw::ScreenMode::Create {
            const TITLE: &str = "Choose which new power you wish to learn first.";
            let prompt = tree_draw::prompt_draw(
                &view,
                TITLE,
                tree_draw::text_width(TITLE.as_bytes(), &font),
                "BACK",
                tree_draw::text_width(b"BACK", &font),
                "BEGIN",
                tree_draw::text_width(b"BEGIN", &font),
                tree.focus,
            );
            draw.batches.extend(prompt.batches);
            draw.texts.extend(prompt.texts);
        }
        if !view.labels && view.hovered > 0 && (view.hovered as usize) <= tree.nodes.len() {
            let (name, description) = tree.node_text(view.hovered as usize - 1, texts);
            let lines = tree_draw::text_split(description, tree_draw::TIP_TEXT_WIDTH, scale, &font);
            let tooltip = tree_draw::tooltip_draw(
                &view,
                &lines,
                &String::from_utf8_lossy(name),
                tree_draw::text_step(&font),
                tree_draw::text_width(name, &font),
            );
            draw.batches.extend(tooltip.batches);
            draw.texts.extend(tooltip.texts);
            // The caption's fade ends 200 ms after the focus leaves; the
            // draw then clears the shown choice, as the original does after
            // drawing the description for the last frame.
            if !view.still_hovering
                && tree.shown_choice > 0
                && now.wrapping_sub(u128::from(tree.choice_stamp)) >= 200
            {
                tree.shown_choice = 0;
            }
        }
        if view.labels {
            let mut names = Vec::with_capacity(tree.nodes.len());
            for node in &tree.nodes {
                let text = if node.name_text == 0 {
                    &[][..]
                } else {
                    texts
                        .get(node.name_text as usize)
                        .map(Vec::as_slice)
                        .unwrap_or_default()
                };
                names.push((
                    String::from_utf8_lossy(text).into_owned(),
                    tree_draw::text_width(text, &font),
                ));
            }
            let labels = tree_draw::labels_draw(&view, &names);
            draw.batches.extend(labels.batches);
            draw.texts.extend(labels.texts);
        }
        if let Some(dragged) = drag.clone() {
            draw.batches.extend(
                tree_draw::preview_draw(
                    dragged.power,
                    dragged.flags,
                    dragged.at[0],
                    dragged.at[1],
                    scale,
                    tree_draw::power_domain(dragged.power),
                    tree.domain_cells
                        .get(usize::from(tree_draw::power_domain(dragged.power)))
                        .copied()
                        .unwrap_or(0),
                )
                .batches,
            );
        }
        draw
    } else {
        tree_draw::bar_draw(&view)
    };
    (draw, canvas_height)
}

/// Plays the UI sound an action queued, through the original's voice
/// environment: the tree's sounds are 2D, at the listener's origin, with the
/// volume and pitch the action names. A missing sound is remembered so the
/// pack is not searched again.
fn ui_sounds(
    game: Option<Res<Game>>,
    mut screen: ResMut<PowerScreen>,
    bank: Option<ResMut<SoundBank>>,
    sources: Option<ResMut<Assets<AudioSource>>>,
    env: Option<ResMut<AudioEnvironment>>,
    mut loaded: Local<HashMap<&'static str, Option<u32>>>,
) {
    let Some(sound) = screen.sound.take() else {
        return;
    };
    let Some(game) = game else {
        return;
    };
    let (Some(mut bank), Some(mut sources), Some(mut env)) = (bank, sources, env) else {
        return;
    };
    let buffer = *loaded
        .entry(sound.name)
        .or_insert_with(|| bank.load_buffer(&game, &mut sources, sound.name));
    if let Some(buffer) = buffer {
        env.play_buffer(buffer, Vec3::ZERO, sound.volume, sound.pitch, 0);
    }
}

/// Wraps a description at `width` characters, keeping its text.
pub fn wrap_text(text: &[u8], width: usize) -> Vec<String> {
    let text = String::from_utf8_lossy(text);
    let mut lines = Vec::new();
    for paragraph in text.split("\r\n") {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split(' ') {
            if !line.is_empty() && line.len() + 1 + word.len() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

/// A rectangle of canvas pixels, both corners included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Region {
    /// The whole `width` by `height` canvas.
    pub fn whole(width: u32, height: u32) -> Self {
        Self {
            left: 0,
            top: 0,
            right: width as i32 - 1,
            bottom: height as i32 - 1,
        }
    }

    fn pixels(&self) -> usize {
        if self.right < self.left || self.bottom < self.top {
            return 0;
        }
        (self.right - self.left + 1) as usize * (self.bottom - self.top + 1) as usize
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.left <= other.right + 1
            && other.left <= self.right + 1
            && self.top <= other.bottom + 1
            && other.top <= self.bottom + 1
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

/// The canvas rasteriser: every batch's triangles with the program's
/// fragment arithmetic, then the texts. The canvas holds premultiplied
/// colour, as the original's blending leaves it in a transparent target.
pub fn render(
    draw: &TreeDraw,
    sheets: &TreeSheets,
    font: Option<&GameFont>,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut canvas = vec![0u8; width as usize * height as usize * 4];
    // The frame program also reads its sheet at mip level 4.
    let coarse = sheets.frames.as_ref().map(level_four);
    render_region(
        &mut canvas,
        draw,
        sheets,
        coarse.as_ref(),
        font,
        (width, height),
        Region::whole(width, height),
    );
    canvas
}

/// [`render`] into the pixels of `region` only, which start transparent;
/// `coarse` is the frames sheet at mip level 4. The region's rows are cut
/// into bands drawn side by side on the compute pool; a pixel is drawn by
/// one band with every batch in order, so the bytes are the same.
fn render_region(
    canvas: &mut [u8],
    draw: &TreeDraw,
    sheets: &TreeSheets,
    coarse: Option<&RgbaImage>,
    font: Option<&GameFont>,
    (width, height): (u32, u32),
    region: Region,
) {
    /// The fewest rows worth a band of their own.
    const BAND_ROWS: usize = 16;
    let row_bytes = width as usize * 4;
    let rows =
        &mut canvas[region.top as usize * row_bytes..(region.bottom as usize + 1) * row_bytes];
    let row_count = (region.bottom - region.top + 1) as usize;
    let pool = ComputeTaskPool::get_or_init(TaskPool::default);
    let bands = pool.thread_num().max(1).min(row_count.div_ceil(BAND_ROWS));
    let band_rows = row_count.div_ceil(bands);
    let draw_band = |index: usize, pixels: &mut [u8]| {
        let top = region.top + (index * band_rows) as i32;
        let band = Region {
            top,
            bottom: top + (pixels.len() / row_bytes) as i32 - 1,
            ..region
        };
        let mut rows = Rows {
            pixels,
            width,
            height,
            top,
        };
        draw_rows(&mut rows, draw, sheets, coarse, font, band);
    };
    if bands == 1 {
        draw_band(0, rows);
        return;
    }
    pool.scope(|scope| {
        for (index, pixels) in rows.chunks_mut(band_rows * row_bytes).enumerate() {
            let draw_band = &draw_band;
            scope.spawn(async move { draw_band(index, pixels) });
        }
    });
}

/// Rows of a `width` by `height` canvas: `pixels` holds the rows from
/// `top` on.
struct Rows<'a> {
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
    top: i32,
}

impl Rows<'_> {
    /// The byte offset of pixel `(x, y)`, or `None` off the canvas or the
    /// rows held.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < self.top || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let at = ((y - self.top) as usize * self.width as usize + x as usize) * 4;
        (at + 4 <= self.pixels.len()).then_some(at)
    }
}

/// Every batch and then the texts into the pixels of `region`, all held by
/// `rows`; they start transparent.
fn draw_rows(
    rows: &mut Rows,
    draw: &TreeDraw,
    sheets: &TreeSheets,
    coarse: Option<&RgbaImage>,
    font: Option<&GameFont>,
    region: Region,
) {
    for y in region.top..=region.bottom {
        let (Some(first), Some(last)) = (rows.index(region.left, y), rows.index(region.right, y))
        else {
            continue;
        };
        rows.pixels[first..last + 4].fill(0);
    }
    for batch in &draw.batches {
        let texture = texture_of(batch.texture, sheets);
        let level = if batch.texture == Texture::Frames {
            coarse
        } else {
            None
        };
        raster_batch(rows, batch, texture, level, region);
    }
    // The runs are the layout's metrics; their glyph quads are in the
    // batches of the skin's text program when the atlas is loaded. The
    // installed packs do not ship the GUI font atlas, so without it the
    // runs are drawn with the game's bitmap font instead.
    if sheets.font.is_none()
        && let Some(font) = font
    {
        for run in &draw.texts {
            blit_text(rows, run, font, region);
        }
    }
}

/// The pixels a batch's triangles can touch: the union of the boxes the
/// rasteriser walks, or `None` when it walks none.
fn batch_region(batch: &Batch, width: u32, height: u32) -> Option<Region> {
    let mut out: Option<Region> = None;
    for triangle in batch.vertices.as_chunks::<3>().0 {
        let [a, b, c] = *triangle;
        let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        if area.abs() < 1e-6 {
            continue;
        }
        let region = Region {
            left: a.x.min(b.x).min(c.x).floor().max(0.0) as i32,
            right: a.x.max(b.x).max(c.x).ceil().min(width as f32 - 1.0) as i32,
            top: a.y.min(b.y).min(c.y).floor().max(0.0) as i32,
            bottom: a.y.max(b.y).max(c.y).ceil().min(height as f32 - 1.0) as i32,
        };
        if region.pixels() == 0 {
            continue;
        }
        out = Some(out.map_or(region, |out| out.union(&region)));
    }
    out
}

/// The screen's canvas between redraws: the last draw, its premultiplied
/// picture and that picture with straight alpha, which the image node
/// blends.
///
/// A redraw compares the new draw with the last batch by batch. A pixel
/// outside every box a changed batch walks (its old and its new triangles)
/// is covered by the same batches in the same order, so it keeps its
/// value; only the changed batches' boxes are cleared and drawn again with
/// every batch, which gives the bytes a whole redraw gives. The tree, its
/// links and the bar hold still while the domain wheel's bands shimmer, so
/// a frame of the open tree redraws the wheel and little else.
#[derive(Default)]
pub struct CanvasCache {
    size: (u32, u32),
    wheel_unit: i32,
    last: Option<TreeDraw>,
    premultiplied: Vec<u8>,
    straight: Vec<u8>,
}

impl CanvasCache {
    /// Forgets the last draw, so the next redraw draws everything (after
    /// the sheets change).
    pub fn invalidate(&mut self) {
        self.last = None;
    }

    /// The picture with premultiplied colour, as [`render`] gives it.
    pub fn premultiplied(&self) -> &[u8] {
        &self.premultiplied
    }

    /// The picture with straight alpha, for the image node.
    pub fn straight(&self) -> &[u8] {
        &self.straight
    }

    /// Brings the canvas to `draw` and returns the regions it redrew
    /// (none when nothing changed). `coarse` is the frames sheet at mip
    /// level 4.
    pub fn update(
        &mut self,
        draw: TreeDraw,
        sheets: &TreeSheets,
        coarse: Option<&RgbaImage>,
        font: Option<&GameFont>,
        (width, height): (u32, u32),
    ) -> Vec<Region> {
        let whole = Region::whole(width, height);
        let texts_drawn = sheets.font.is_none() && font.is_some();
        let regions = match &self.last {
            Some(last)
                if self.size == (width, height)
                    && self.wheel_unit == sheets.wheel_unit
                    && (!texts_drawn || last.texts == draw.texts) =>
            {
                changed_regions(&last.batches, &draw.batches, width, height)
            }
            _ => vec![whole],
        };
        if self.size != (width, height) {
            let bytes = width as usize * height as usize * 4;
            self.premultiplied = vec![0; bytes];
            self.straight = vec![0; bytes];
            self.size = (width, height);
        }
        self.wheel_unit = sheets.wheel_unit;
        for region in &regions {
            render_region(
                &mut self.premultiplied,
                &draw,
                sheets,
                coarse,
                font,
                (width, height),
                *region,
            );
            straighten(&self.premultiplied, &mut self.straight, width, *region);
        }
        self.last = Some(draw);
        regions
    }
}

/// The regions the batches that differ between `old` and `new` can touch,
/// with overlapping ones merged; the whole canvas when they would cover
/// half of it.
fn changed_regions(old: &[Batch], new: &[Batch], width: u32, height: u32) -> Vec<Region> {
    let whole = Region::whole(width, height);
    let mut regions: Vec<Region> = Vec::new();
    for index in 0..old.len().max(new.len()) {
        let (before, after) = (old.get(index), new.get(index));
        if before == after {
            continue;
        }
        for batch in [before, after].into_iter().flatten() {
            let Some(mut region) = batch_region(batch, width, height) else {
                continue;
            };
            // Merge until no kept region overlaps the new one.
            while let Some(at) = regions.iter().position(|kept| kept.overlaps(&region)) {
                region = region.union(&regions.swap_remove(at));
            }
            regions.push(region);
        }
    }
    let total: usize = regions.iter().map(Region::pixels).sum();
    if total * 2 >= whole.pixels() {
        return vec![whole];
    }
    regions
}

/// Copies `region` of a premultiplied canvas into `out` with straight
/// alpha: each colour over its alpha, rounded; a transparent pixel is
/// black.
fn straighten(canvas: &[u8], out: &mut [u8], width: u32, region: Region) {
    for y in region.top..=region.bottom {
        let row = y as usize * width as usize;
        for x in region.left..=region.right {
            let at = (row + x as usize) * 4;
            let alpha = u32::from(canvas[at + 3]);
            if alpha == 0 {
                out[at..at + 4].fill(0);
                continue;
            }
            for channel in 0..3 {
                let colour = u32::from(canvas[at + channel]);
                out[at + channel] = ((colour * 255 + alpha / 2) / alpha).min(255) as u8;
            }
            out[at + 3] = alpha as u8;
        }
    }
}

fn texture_of(texture: Texture, sheets: &TreeSheets) -> Option<&RgbaImage> {
    match texture {
        Texture::None => None,
        Texture::Domain(domain) => sheets
            .domains
            .get(usize::from(domain))
            .and_then(Option::as_ref),
        Texture::Frames => sheets.frames.as_ref(),
        Texture::Mods => sheets.mods.as_ref(),
        Texture::Flamelets => sheets.flamelets.as_ref(),
        Texture::Wheel => sheets.wheel.as_ref(),
        Texture::Font(_) => sheets.font.as_ref(),
    }
}

/// The picture at mip level 4: every 16 by 16 block averaged, at least one
/// texel a side.
fn level_four(image: &RgbaImage) -> RgbaImage {
    let width = (image.width >> 4).max(1);
    let height = (image.height >> 4).max(1);
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            let (x0, y0) = (x * image.width / width, y * image.height / height);
            let (x1, y1) = (
                ((x + 1) * image.width / width).max(x0 + 1),
                ((y + 1) * image.height / height).max(y0 + 1),
            );
            for sy in y0..y1.min(image.height) {
                for sx in x0..x1.min(image.width) {
                    let at = (sy as usize * image.width as usize + sx as usize) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u32::from(image.pixels[at + channel]);
                    }
                    count += 1;
                }
            }
            for total in sum {
                pixels.push((total / count.max(1)) as u8);
            }
        }
    }
    RgbaImage {
        width,
        height,
        pixels,
    }
}

/// Samples a texture at `(u, v)` with bilinear filtering, the way the
/// original's GL sampler does. The image's rows are bottom-up.
fn sample(image: Option<&RgbaImage>, u: f32, v: f32) -> [f32; 4] {
    sample_with(image, u, v, false)
}

/// [`sample`] repeating the picture on both axes, as the flamelets sheet's
/// wrap flags ask; the wheel's programs scroll across it.
fn sample_repeat(image: Option<&RgbaImage>, u: f32, v: f32) -> [f32; 4] {
    sample_with(image, u, v, true)
}

fn sample_with(image: Option<&RgbaImage>, u: f32, v: f32, repeat: bool) -> [f32; 4] {
    let Some(image) = image else {
        return [1.0, 1.0, 1.0, 1.0];
    };
    if image.width == 0 || image.height == 0 {
        return [1.0, 1.0, 1.0, 1.0];
    }
    let x = u * image.width as f32 - 0.5;
    let y = (1.0 - v) * image.height as f32 - 0.5;
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;
    let fetch = |x: i32, y: i32| {
        let (x, y) = if repeat {
            (
                x.rem_euclid(image.width as i32) as usize,
                y.rem_euclid(image.height as i32) as usize,
            )
        } else {
            (
                x.clamp(0, image.width as i32 - 1) as usize,
                y.clamp(0, image.height as i32 - 1) as usize,
            )
        };
        let at = (y * image.width as usize + x) * 4;
        [
            f32::from(image.pixels[at]) / 255.0,
            f32::from(image.pixels[at + 1]) / 255.0,
            f32::from(image.pixels[at + 2]) / 255.0,
            f32::from(image.pixels[at + 3]) / 255.0,
        ]
    };
    let x0 = x0 as i32;
    let y0 = y0 as i32;
    let a = fetch(x0, y0);
    let b = fetch(x0 + 1, y0);
    let c = fetch(x0, y0 + 1);
    let d = fetch(x0 + 1, y0 + 1);
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let top = [
        mix(a[0], b[0], fx),
        mix(a[1], b[1], fx),
        mix(a[2], b[2], fx),
        mix(a[3], b[3], fx),
    ];
    let bottom = [
        mix(c[0], d[0], fx),
        mix(c[1], d[1], fx),
        mix(c[2], d[2], fx),
        mix(c[3], d[3], fx),
    ];
    [
        mix(top[0], bottom[0], fy),
        mix(top[1], bottom[1], fy),
        mix(top[2], bottom[2], fy),
        mix(top[3], bottom[3], fy),
    ]
}

/// One batch's triangles, inside `clip`. Each row of a triangle's box tests only the
/// columns [`row_span`] leaves, a run that holds every pixel the inside
/// test passes; the test, its arithmetic and the order the pixels blend in
/// are unchanged, so the picture is the same as testing the whole box.
fn raster_batch(
    rows: &mut Rows,
    batch: &Batch,
    texture: Option<&RgbaImage>,
    coarse: Option<&RgbaImage>,
    clip: Region,
) {
    for triangle in batch.vertices.as_chunks::<3>().0 {
        let [a, b, c] = *triangle;
        let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        if area.abs() < 1e-6 {
            continue;
        }
        let (width, height) = (rows.width, rows.height);
        let min_x = (a.x.min(b.x).min(c.x).floor().max(0.0) as i32).max(clip.left);
        let max_x = (a.x.max(b.x).max(c.x).ceil().min(width as f32 - 1.0) as i32).min(clip.right);
        let min_y = (a.y.min(b.y).min(c.y).floor().max(0.0) as i32).max(clip.top);
        let max_y = (a.y.max(b.y).max(c.y).ceil().min(height as f32 - 1.0) as i32).min(clip.bottom);
        if min_x > max_x {
            continue;
        }
        for y in min_y..=max_y {
            let py = y as f32 + 0.5;
            let Some((first, last)) = row_span([a, b, c], area, py, min_x, max_x) else {
                continue;
            };
            for x in first..=last {
                let px = x as f32 + 0.5;
                let w0 = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) / area;
                let w1 = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let vertex = mix_vertex(a, b, c, w0, w1, w2);
                let colour = shade(batch, vertex, texture, coarse);
                if batch.blend == exa_ui::wheel_draw::WHEEL_PROGRAM_BLEND {
                    blend_premultiplied(rows, x, y, colour);
                } else {
                    blend(rows, x, y, colour);
                }
            }
        }
    }
}

/// The columns of row `py` (a pixel centre) between `min_x` and `max_x`
/// that may pass the triangle's inside test, or `None` when none can.
///
/// Along a row each barycentric weight is a line in the column, so each
/// edge keeps the columns on one side of a root. The test computes the
/// weights in single precision from the absolute coordinates, so its sign
/// can be wrong only where the exact weight's numerator is within a few
/// rounding errors of the products' magnitudes; each root is widened by
/// that error over the edge's slope, plus two pixels. A row whose numbers
/// are not finite keeps the whole box, as the test then may pass anywhere.
fn row_span(
    [a, b, c]: [TreeVertex; 3],
    area: f32,
    py: f32,
    min_x: i32,
    max_x: i32,
) -> Option<(i32, i32)> {
    // Each weight's numerator is `offset + px * slope`, after the edge
    // opposite its vertex.
    let edges = [
        (b.x * (c.y - py) - (b.y - py) * c.x, b.y - c.y, b, c),
        (c.x * (a.y - py) - (c.y - py) * a.x, c.y - a.y, c, a),
        (a.x * (b.y - py) - (a.y - py) * b.x, a.y - b.y, a, b),
    ];
    let reach = (min_x.unsigned_abs().max(max_x.unsigned_abs()) + 1) as f32;
    let size = |p: TreeVertex, q: TreeVertex| {
        (p.x.abs() + reach) * (q.y.abs() + py.abs()) + (p.y.abs() + py.abs()) * (q.x.abs() + reach)
    };
    let magnitude: f32 = edges.iter().map(|&(_, _, p, q)| size(p, q)).sum();
    let error = (16.0 * magnitude + 4.0 * area.abs()) * f32::EPSILON;
    let (mut first, mut last) = (min_x as f32, max_x as f32);
    for (offset, slope, _, _) in edges {
        // The weight's sign follows the area's.
        let (offset, slope) = if area < 0.0 {
            (-offset, -slope)
        } else {
            (offset, slope)
        };
        if !(offset.is_finite() && slope.is_finite() && error.is_finite()) {
            return Some((min_x, max_x));
        }
        if slope.abs() * reach <= error {
            // Nearly level along the row: either everywhere or nowhere.
            if offset + slope.abs() * reach < -error {
                return None;
            }
            continue;
        }
        // The column centre `px = x + 0.5` where the numerator is zero.
        let root = -offset / slope - 0.5;
        let widen = error / slope.abs() + 2.0;
        if slope > 0.0 {
            first = first.max(root - widen);
        } else {
            last = last.min(root + widen);
        }
    }
    if first > last {
        return None;
    }
    Some((
        first.floor().max(min_x as f32) as i32,
        last.ceil().min(max_x as f32) as i32,
    ))
}

fn mix_vertex(
    a: TreeVertex,
    b: TreeVertex,
    c: TreeVertex,
    wa: f32,
    wb: f32,
    wc: f32,
) -> TreeVertex {
    let mix = |x: f32, y: f32, z: f32| x * wa + y * wb + z * wc;
    TreeVertex {
        x: mix(a.x, b.x, c.x),
        y: mix(a.y, b.y, c.y),
        u: mix(a.u, b.u, c.u),
        v: mix(a.v, b.v, c.v),
        u2: mix(a.u2, b.u2, c.u2),
        v2: mix(a.v2, b.v2, c.v2),
        colour: [
            mix(a.colour[0], b.colour[0], c.colour[0]),
            mix(a.colour[1], b.colour[1], c.colour[1]),
            mix(a.colour[2], b.colour[2], c.colour[2]),
            mix(a.colour[3], b.colour[3], c.colour[3]),
        ],
    }
}

/// The programs' `clamp(x, 0, 1)` is a max then a min, so a not-a-number
/// becomes zero rather than staying a not-a-number.
#[allow(clippy::manual_clamp)]
fn clamp01(x: f32) -> f32 {
    x.max(0.0).min(1.0)
}

/// The six fragment programs' arithmetic.
fn shade(
    batch: &Batch,
    vertex: TreeVertex,
    texture: Option<&RgbaImage>,
    coarse: Option<&RgbaImage>,
) -> [f32; 4] {
    let uniforms = &batch.uniforms;
    let col = vertex.colour;
    let mut out = match batch.program {
        Program::Ring => {
            let tx = vertex.u - 0.5;
            let ty = vertex.v - 0.5;
            let a = (tx * tx + ty * ty) * 5.0;
            let mut b = a + 0.2;
            b *= b;
            b = (b * b).min(1.0);
            let d = b + 0.4;
            let mut a2 = (a - 0.8) * 5.0;
            let mut e = 1.0 - a2;
            b = clamp01((a2 + 0.6) * 3.0);
            a2 = clamp01(e * 3.0);
            let rgb = [col[0] * b * a2, col[1] * b * a2, col[2] * b * a2];
            e = clamp01(e * 0.6 + 0.8);
            e = clamp01(e * e * d);
            [rgb[0], rgb[1], rgb[2], col[3] * e]
        }
        Program::Mods => {
            let t = [vertex.u, vertex.v];
            let texel = |du: f32, dv: f32| sample(texture, t[0] + du, t[1] + dv)[0];
            let mut a = texel(uniforms.par0[0], 0.0);
            a += texel(-uniforms.par0[0], 0.0);
            a += texel(0.0, uniforms.par0[1]);
            a += texel(0.0, -uniforms.par0[1]);
            a *= 0.25;
            let b = sample(texture, t[0], t[1])[0];
            a = clamp01(a + b);
            [col[0] * b, col[1] * b, col[2] * b, col[3] * a]
        }
        Program::Icon => {
            let tx = vertex.u - uniforms.par0[0];
            let ty = vertex.v - uniforms.par0[1];
            let a = (tx * uniforms.tscl[0]) * (tx * uniforms.tscl[0])
                + (ty * uniforms.tscl[1]) * (ty * uniforms.tscl[1]);
            let mut b = clamp01((a + uniforms.par1[0]) * uniforms.par1[1]);
            let a2 = 1.0 - a * 83.0;
            let c = clamp01(a2 * uniforms.par0[3]);
            let su = uniforms.par0[0] + tx * uniforms.par0[2];
            let sv = uniforms.par0[1] + ty * uniforms.par0[2];
            let tex = sample(texture, su, sv);
            let d = clamp01(a2 + uniforms.par1[2]);
            b = clamp01(b * 0.85 + 0.15);
            let mut aa = 1.0 - a2;
            aa = aa * aa * 0.3 * c;
            let c2 = c * b + aa;
            [tex[0] * d, tex[1] * d, tex[2] * d, col[3] * c2]
        }
        Program::Frame => {
            // The band keeps the bowed row only; the texture's row adds the
            // pad terms on the unbowed one.
            let b = vertex.u2;
            let a = 0.5 - b;
            let a = 0.25 - a * a;
            let band_row = vertex.v - a * 0.025;
            let tx = vertex.u;
            let ty = vertex.v + a * uniforms.par0[2] + uniforms.par0[1];
            let lower = clamp01(band_row * 16.0);
            let upper = clamp01(4.0 - band_row * 16.0);
            let band = lower * upper;
            let tex = sample(texture, tx, ty)[0];
            let lod = sample(coarse, tx, ty)[0];
            let mut c = ((0.13 - vertex.v).abs() * 16.0).min(1.0);
            c = 1.0 - c * c;
            let a2 = (tex + lod * c) * (1.0 - c * 0.65);
            let col2 = [
                uniforms.frame_col[0] + col[0] * c * 1.5,
                uniforms.frame_col[1] + col[1] * c * 1.5,
                uniforms.frame_col[2] + col[2] * c * 1.5,
            ];
            let rgb = [
                col2[0] * a2 * col[3],
                col2[1] * a2 * col[3],
                col2[2] * a2 * col[3],
            ];
            let inner = a2 * uniforms.par0[3] + (1.0 - uniforms.par0[3]);
            [rgb[0], rgb[1], rgb[2], col[3] * band * inner * 0.63]
        }
        Program::Link | Program::LinkGlow => {
            // The ring program: two taps of the flamelets sheet, the first
            // along the turned y and the second a quarter away, with the
            // soft program raising the first to the eighth power.
            let b = vertex.u2;
            let a0 = 0.5 - b;
            let a0 = 0.25 - a0 * a0;
            let mut ty = vertex.v * 0.05 + uniforms.par0[2] * a0;
            let bb = vertex.v + ty * 2.0;
            ty *= uniforms.par0[0];
            let uy = (ty + uniforms.par0[1]) * 0.25;
            ty += uniforms.par0[1];
            let ux = vertex.u - uniforms.par0[1];
            let mut c = sample(texture, vertex.u, ty)[0];
            let mut a = 1.0 - bb;
            a *= a;
            if batch.program == Program::Link {
                a *= a;
                a *= a;
            }
            c += a;
            let d = sample(texture, ux, uy)[0];
            c = c * d * 0.75;
            if batch.program == Program::LinkGlow {
                c = c + (a - c) * 0.75;
            }
            let mask = clamp01(bb * 32.0);
            let under = clamp01(c - (bb * bb + uniforms.par0[3]));
            let alpha = mask * under * under;
            if batch.program == Program::Link {
                [col[0] * 1.25, col[1] * 1.25, col[2] * 1.25, col[3] * alpha]
            } else {
                [
                    col[0] * col[0] * 0.75,
                    col[1] * col[1] * 0.75,
                    col[2] * col[2] * 0.75,
                    col[3] * alpha,
                ]
            }
        }
        Program::Spline => {
            // The spline renderer's ribbon: the row coordinate's fraction
            // shapes the cross-section and the join shifts its middle.
            let row = vertex.u - vertex.u.floor();
            let across = (0.5 - row).abs();
            let profile = (1.0 - across * 2.0).max(0.0).sqrt();
            let middle = clamp01(0.5 + profile * vertex.u2);
            let band = 0.5 - (vertex.v - middle).abs();
            let wide = clamp01(band * uniforms.par0[0] + uniforms.par0[1]);
            let soft = clamp01(band * uniforms.par0[2] * 0.2);
            let alpha = clamp01(soft + wide);
            [
                col[0] * wide,
                col[1] * wide,
                col[2] * wide,
                col[3] * alpha * alpha,
            ]
        }
        Program::WheelBack => exa_ui::wheel_draw::back_fragment(
            [vertex.u, vertex.v],
            col,
            uniforms.par0[0],
            uniforms.par1[0],
            |u, v| sample_repeat(texture, u, v)[0],
        ),
        Program::WheelGlow => exa_ui::wheel_draw::glow_fragment(
            [vertex.u, vertex.v],
            col,
            uniforms.par0[0],
            |u, v| sample_repeat(texture, u, v)[0],
        ),
        Program::WheelSector => exa_ui::wheel_draw::sector_fragment(
            [vertex.u, vertex.v],
            col,
            uniforms.par0[0],
            [uniforms.par1[0], uniforms.par1[1], uniforms.par1[2]],
            |u, v| sample_repeat(texture, u, v)[0],
        ),
        Program::Rounded => {
            exa_ui::wheel_draw::rounded_fragment([vertex.u, vertex.v], col, uniforms.par0)
        }
        Program::SkinText => {
            // The skin's textured program with `TexCol` one: the atlas
            // colour multiplies the run's ink.
            let tex = sample(texture, vertex.u, vertex.v);
            let tex_col = uniforms.par0[0];
            [
                col[0] * (tex[0] * tex_col + 1.0 - tex_col),
                col[1] * (tex[1] * tex_col + 1.0 - tex_col),
                col[2] * (tex[2] * tex_col + 1.0 - tex_col),
                col[3] * tex[3],
            ]
        }
    };
    out.iter_mut().for_each(|v| *v = clamp01(*v));
    out
}

/// The blend word `0x103`: the source colour as it is over the destination
/// scaled by one minus the source alpha.
fn blend_premultiplied(rows: &mut Rows, x: i32, y: i32, colour: [f32; 4]) {
    let Some(at) = rows.index(x, y) else {
        return;
    };
    let canvas = &mut *rows.pixels;
    let a = colour[3];
    for (index, channel) in colour.iter().take(3).enumerate() {
        let dst = f32::from(canvas[at + index]);
        canvas[at + index] = (channel * 255.0 + dst * (1.0 - a)).clamp(0.0, 255.0) as u8;
    }
    let dst_a = f32::from(canvas[at + 3]) / 255.0;
    canvas[at + 3] = ((a + dst_a * (1.0 - a)) * 255.0).clamp(0.0, 255.0) as u8;
}

fn blend(rows: &mut Rows, x: i32, y: i32, colour: [f32; 4]) {
    let Some(at) = rows.index(x, y) else {
        return;
    };
    let a = colour[3];
    if a <= 0.0 {
        return;
    }
    let canvas = &mut *rows.pixels;
    for (index, channel) in colour.iter().take(3).enumerate() {
        let src = channel * 255.0;
        let dst = f32::from(canvas[at + index]);
        canvas[at + index] = (src * a + dst * (1.0 - a)).clamp(0.0, 255.0) as u8;
    }
    let dst_a = f32::from(canvas[at + 3]) / 255.0;
    canvas[at + 3] = ((a + dst_a * (1.0 - a)) * 255.0).clamp(0.0, 255.0) as u8;
}

/// Blits one text run with the game font, tinted, inside `clip`.
fn blit_text(rows: &mut Rows, run: &TextRun, font: &GameFont, clip: Region) {
    let rendered = font.render(&run.text);
    if rendered.width == 0 || rendered.height == 0 {
        return;
    }
    let scale = run.size / font.line_height() as f32;
    for y in 0..rendered.height {
        for x in 0..rendered.width {
            let alpha = rendered.alpha[y * rendered.width + x];
            if alpha == 0 {
                continue;
            }
            let px = run.x as i32 + (x as f32 * scale) as i32;
            let py = run.y as i32 - ((rendered.height - y) as f32 * scale) as i32;
            let colour = [
                run.colour[0],
                run.colour[1],
                run.colour[2],
                run.colour[3] * f32::from(alpha) / 255.0,
            ];
            for oy in 0..(scale.ceil() as i32).max(1) {
                for ox in 0..(scale.ceil() as i32).max(1) {
                    if clip.contains(px + ox, py + oy) {
                        blend(rows, px + ox, py + oy, colour);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ui::tree_draw::Uniforms;

    /// The plugin's systems run three updates without a window or a level.
    #[test]
    fn the_power_screen_plugin_runs_without_a_window() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<bevy::input::mouse::MouseWheel>()
            .add_plugins(PowerScreenPlugin);
        for _ in 0..3 {
            app.update();
        }
    }

    #[test]
    fn the_tree_font_follows_the_interface_unit() {
        // Unit four is level one: Rayform08's metrics over the base atlas.
        let (font, size) = tree_font(4, Some(("fontbase24r", (1024, 1024))));
        assert_eq!((font.texture, font.glyph), ("fontbase24r", (11, 16)));
        assert_eq!(size, (313, 341));
        // Unit five is level two: Rayform10's metrics, a fifth larger.
        let (font, size) = tree_font(5, Some(("fontbase24r", (1024, 1024))));
        assert_eq!(font.glyph, (15, 20));
        assert_eq!(size, (427, 427));
        // No atlas emits no glyphs; the base font's runs still draw.
        assert_eq!(tree_font(4, None).1, (0, 0));
        let sheets = TreeSheets {
            font: Some(RgbaImage {
                width: 1,
                height: 1,
                pixels: vec![0; 4],
            }),
            ..TreeSheets::default()
        };
        assert_eq!(
            texture_of(Texture::Font("fontbase24r"), &sheets).map(|i| i.width),
            Some(1)
        );
    }

    #[test]
    fn a_domain_sheet_gives_its_cell_count() {
        let sheet = |width| RgbaImage {
            width,
            height: width,
            pixels: Vec::new(),
        };
        let mut sheets = TreeSheets::default();
        // The two sizes the packs carry: four 512-pixel cells or one.
        sheets.domains[1] = Some(sheet(2048));
        sheets.domains[4] = Some(sheet(512));
        assert_eq!(domain_cells(&sheets), [0, 4, 0, 0, 1, 0, 0]);
        // A sheet smaller than one cell gives no icons.
        sheets.domains[2] = Some(sheet(256));
        assert_eq!(domain_cells(&sheets)[2], 0);
    }

    #[test]
    fn the_ring_shader_draws_a_dot() {
        let mut batch = Batch::new(Program::Ring, Texture::None);
        batch.vertices = vec![TreeVertex::new(0.0, 0.0, 0.5, 0.5)];
        let centre = shade(&batch, batch.vertices[0], None, None);
        assert!(centre[3] > 0.0);
        let edge = TreeVertex::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(shade(&batch, edge, None, None)[3], 0.0);
    }

    #[test]
    fn the_icon_mask_fades_at_the_cell_edge() {
        let mut batch = Batch::new(Program::Icon, Texture::None);
        batch.uniforms = Uniforms {
            par0: [0.5, 0.5, 1.0, 8.5],
            par1: [1.0, 1.0, 0.5, 0.0],
            ..Uniforms::default()
        };
        let centre = TreeVertex::new(0.0, 0.0, 0.5, 0.5).with_colour([1.0, 1.0, 1.0, 1.0]);
        let middle = shade(&batch, centre, None, None);
        assert!(middle[3] > 0.9);
        let edge = TreeVertex::new(0.0, 0.0, 0.0, 0.0).with_colour([1.0, 1.0, 1.0, 1.0]);
        assert!(shade(&batch, edge, None, None)[3] < middle[3]);
    }

    #[test]
    fn each_domain_draws_with_its_own_sheet() {
        let sheet = |width| RgbaImage {
            width,
            height: 1,
            pixels: vec![0; width as usize * 4],
        };
        let mut sheets = TreeSheets::default();
        sheets.domains[1] = Some(sheet(1));
        sheets.domains[2] = Some(sheet(2));
        assert_eq!(
            texture_of(Texture::Domain(2), &sheets).map(|i| i.width),
            Some(2)
        );
        assert_eq!(
            texture_of(Texture::Domain(1), &sheets).map(|i| i.width),
            Some(1)
        );
        assert!(texture_of(Texture::Domain(9), &sheets).is_none());
    }

    #[test]
    fn the_coarse_level_averages_sixteen_texel_blocks() {
        let mut image = RgbaImage {
            width: 32,
            height: 16,
            pixels: vec![0; 32 * 16 * 4],
        };
        for y in 0..16usize {
            for x in 16..32usize {
                image.pixels[(y * 32 + x) * 4] = 200;
            }
        }
        let level = level_four(&image);
        assert_eq!((level.width, level.height), (2, 1));
        assert_eq!((level.pixels[0], level.pixels[4]), (0, 200));
    }

    #[test]
    fn descriptions_wrap_and_keep_their_breaks() {
        let lines = wrap_text(b"one two three four\r\n\r\nfive", 7);
        assert_eq!(lines, vec!["one two", "three", "four", "", "five"]);
        let lines = wrap_text(b"short", 10);
        assert_eq!(lines, vec!["short"]);
    }

    #[test]
    fn the_canvas_draws_and_blends() {
        let mut draw = TreeDraw::default();
        let mut batch = Batch::new(Program::Ring, Texture::None);
        batch.vertices = vec![
            TreeVertex::new(0.0, 0.0, 0.5, 0.5).with_colour([1.0, 0.0, 0.0, 1.0]),
            TreeVertex::new(10.0, 0.0, 0.5, 0.5).with_colour([1.0, 0.0, 0.0, 1.0]),
            TreeVertex::new(10.0, 10.0, 0.5, 0.5).with_colour([1.0, 0.0, 0.0, 1.0]),
        ];
        draw.batches.push(batch);
        let pixels = render(&draw, &TreeSheets::default(), None, 16, 16);
        assert_eq!(pixels.len(), 16 * 16 * 4);
        // A corner outside the triangle is transparent.
        assert_eq!(pixels[15 * 16 * 4 + 3], 0);
    }

    /// The installed game, when `EXANIMA_DIR` (or the usual places) has one.
    fn installed_game() -> Option<Game> {
        exa_assets::locate_install(None)
            .and_then(|dir| exa_assets::GameData::open(&dir).ok())
            .map(|data| Game(Arc::new(data)))
    }

    /// A 1280 by 720 app with the screen, the simulation keys and a 16 ms
    /// clock, pressing each `(frame, key)` the way `--press` does (after the
    /// input systems, released the next frame).
    fn scripted_tree_app(game: Option<&Game>, presses: Vec<(u32, KeyCode)>) -> App {
        use bevy::asset::AssetPlugin;
        use bevy::input::InputPlugin;
        use bevy::time::TimeUpdateStrategy;
        use bevy::window::WindowResolution;
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            InputPlugin,
            exa_physics::body::BodyPlugin,
            crate::sim_control::SimControlPlugin,
        ))
        .init_asset::<Image>()
        .init_resource::<Bindings>()
        .insert_resource(exa_physics::Physics::default())
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(16),
        ));
        if let Some(game) = game {
            app.insert_resource(game.clone());
        }
        app.world_mut().spawn((
            Window {
                resolution: WindowResolution::new(1280, 720),
                ..default()
            },
            PrimaryWindow,
        ));
        app.add_plugins(PowerScreenPlugin);
        app.add_systems(
            PreUpdate,
            (move |mut keys: ResMut<ButtonInput<KeyCode>>, mut frame: Local<u32>| {
                *frame += 1;
                for &(at, key) in &presses {
                    if *frame == at {
                        keys.press(key);
                    } else if *frame == at + 1 {
                        keys.release(key);
                    }
                }
            })
            .after(bevy::input::InputSystems),
        );
        app
    }

    /// The tree's key, pressed the way `--press` presses it (after the
    /// input systems, released the frame after) in an app that also runs
    /// the simulation keys, opens the screen: the game clock keeps running,
    /// the opening fade completes and the canvas is shown. With the
    /// installed packs the picture covers the window (the backdrop) and
    /// draws the wheel at the lower left. Any other key then closes it.
    #[test]
    fn a_scripted_tree_key_opens_and_shows_the_screen() {
        let game = installed_game();
        let key = Bindings::default()
            .key_code(Action::PowerTree)
            .expect("the tree's key");
        // Any other key (frame 110) closes the shown tree.
        let mut app = scripted_tree_app(game.as_ref(), vec![(5, key), (110, KeyCode::KeyW)]);
        // A hundred frames after the press, as the GPU check's screenshot.
        for _ in 0..105 {
            app.update();
        }
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        let screen = app.world().resource::<PowerScreen>();
        assert!(screen.open && screen.bar_open);
        assert_eq!(screen.tree.window_alpha, 1.0);
        assert_eq!(screen.tree.content_alpha, 1.0);
        let mut canvases = app.world_mut().query::<(&ImageNode, &Visibility)>();
        let shown: Vec<_> = canvases
            .iter(app.world())
            .map(|(image, visibility)| (image.image.clone(), *visibility))
            .collect();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].1, Visibility::Inherited);
        // The canvas draws over the HUD's layers.
        let mut layers = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<PowerCanvas>>();
        let layer = layers.single(app.world()).expect("the canvas's layer");
        assert_eq!(layer.0, TREE_Z);
        if game.is_some() {
            let images = app.world().resource::<Assets<Image>>();
            let image = images.get(&shown[0].0).expect("the canvas image");
            assert_eq!((image.width(), image.height()), (1280, 720));
            let data = image.data.as_ref().expect("the pixels");
            let alpha = |x: u32, y: u32| data[((y * 1280 + x) * 4 + 3) as usize];
            let covered = data.chunks(4).filter(|p| p[3] > 8).count();
            assert!(covered > 1280 * 720 / 2, "{covered}");
            // The wheel's icons are lit at the lower left.
            let lit = (440..680)
                .flat_map(|y| (30..270).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    let at = ((y * 1280 + x) * 4) as usize;
                    alpha(x, y) > 8 && data[at..at + 3].iter().any(|&c| c > 64)
                })
                .count();
            assert!(lit > 200, "{lit}");
        }
        // Any other key (W, five frames on) closes the shown tree: after the
        // 256 ms fade-out the screen is off and its canvas hidden.
        for _ in 0..30 {
            app.update();
        }
        let screen = app.world().resource::<PowerScreen>();
        assert!(!screen.open && !screen.bar_open);
        let mut canvases = app.world_mut().query::<&Visibility>();
        assert!(canvases.iter(app.world()).all(|v| *v == Visibility::Hidden));
    }

    /// The open tree's frames as the GPU check takes them (1280 by 720, a
    /// hundred frames after the key): a later frame redraws only the
    /// batches that changed (the wheel's and the shimmering link glow's
    /// boxes), and the kept canvas is byte for byte the whole redraw.
    /// Prints the costs; needs the installed packs.
    #[test]
    fn the_open_tree_redraws_only_what_changed() {
        let Some(game) = installed_game() else {
            return;
        };
        let key = Bindings::default()
            .key_code(Action::PowerTree)
            .expect("the tree's key");
        let mut app = scripted_tree_app(Some(&game), vec![(5, key)]);
        for _ in 0..105 {
            app.update();
        }
        let now = app.world().resource::<Time>().elapsed().as_millis();
        let mut screen = app.world_mut().resource_mut::<PowerScreen>();
        assert!(screen.open);
        let size = (1280, 720);
        let mut cache = CanvasCache::default();
        for (frame, later) in [0u128, 33, 66, 500, 5000].into_iter().enumerate() {
            let (draw, height) = frame_draw(&mut screen, size.0, size.1, 4.0, now + later);
            assert_eq!(height, size.1);
            let started = std::time::Instant::now();
            let whole = render(&draw, &screen.sheets, screen.font.as_ref(), size.0, size.1);
            let whole_cost = started.elapsed();
            let started = std::time::Instant::now();
            let regions = cache.update(
                draw,
                &screen.sheets,
                screen.frames_coarse.as_ref(),
                screen.font.as_ref(),
                size,
            );
            let kept_cost = started.elapsed();
            let pixels: usize = regions.iter().map(Region::pixels).sum();
            eprintln!(
                "+{later} ms: whole render {whole_cost:?}, kept canvas {kept_cost:?} \
                 ({} regions, {pixels} pixels)",
                regions.len()
            );
            assert!(cache.premultiplied() == whole.as_slice());
            if frame == 0 {
                assert_eq!(regions, vec![Region::whole(size.0, size.1)]);
            } else {
                // The wheel and the link glow, well under the window.
                assert!(pixels * 4 < (size.0 * size.1) as usize, "{pixels}");
            }
        }
    }

    /// The whole-box rasteriser [`raster_batch`] replaced: every pixel of
    /// each triangle's box takes the inside test.
    fn raster_batch_whole_box(rows: &mut Rows, batch: &Batch) {
        let (width, height) = (rows.width, rows.height);
        for triangle in batch.vertices.as_chunks::<3>().0 {
            let [a, b, c] = *triangle;
            let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if area.abs() < 1e-6 {
                continue;
            }
            let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
            let max_x = a.x.max(b.x).max(c.x).ceil().min(width as f32 - 1.0) as i32;
            let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
            let max_y = a.y.max(b.y).max(c.y).ceil().min(height as f32 - 1.0) as i32;
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let px = x as f32 + 0.5;
                    let py = y as f32 + 0.5;
                    let w0 = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) / area;
                    let w1 = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let vertex = mix_vertex(a, b, c, w0, w1, w2);
                    let colour = shade(batch, vertex, None, None);
                    blend(rows, x, y, colour);
                }
            }
        }
    }

    /// The row spans keep every pixel: random, long, thin, nearly level,
    /// nearly flat, off-window and both-winding triangles draw the same
    /// bytes as testing the whole box.
    #[test]
    fn row_spans_draw_the_same_pixels_as_the_whole_box() {
        let mut seed = 0x2545_f491_u32;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as f32 / u32::MAX as f32
        };
        let (width, height) = (200u32, 120u32);
        let mut batch = Batch::new(Program::Ring, Texture::None);
        for index in 0..3000 {
            let kind = index % 5;
            let a = [next() * 260.0 - 30.0, next() * 180.0 - 30.0];
            let (b, c) = match kind {
                // Random.
                0 => (
                    [next() * 260.0 - 30.0, next() * 180.0 - 30.0],
                    [next() * 260.0 - 30.0, next() * 180.0 - 30.0],
                ),
                // A long thin sliver, as a link's ribbon.
                1 => {
                    let b = [next() * 260.0 - 30.0, next() * 180.0 - 30.0];
                    let t = next() * 0.6;
                    (b, [a[0] + t, a[1] + t * 0.5 + next() * 0.01])
                }
                // Nearly level.
                2 => (
                    [a[0] + next() * 200.0, a[1] + next() * 0.02],
                    [a[0] + next() * 200.0, a[1] - next() * 3.0],
                ),
                // Nearly upright.
                3 => (
                    [a[0] + next() * 0.02, a[1] + next() * 150.0],
                    [a[0] - next() * 3.0, a[1] + next() * 150.0],
                ),
                // Nearly flat: three points almost on a line.
                _ => {
                    let d = [next() * 100.0 - 50.0, next() * 100.0 - 50.0];
                    let e = next() * 1e-3;
                    (
                        [a[0] + d[0], a[1] + d[1]],
                        [a[0] + d[0] * 2.0 + e, a[1] + d[1] * 2.0 - e],
                    )
                }
            };
            let colour = [next(), next(), next(), 0.2 + next() * 0.5];
            for p in [a, b, c] {
                batch
                    .vertices
                    .push(TreeVertex::new(p[0], p[1], next(), next()).with_colour(colour));
            }
        }
        let rows = |pixels| Rows {
            pixels,
            width,
            height,
            top: 0,
        };
        let mut spans = vec![0u8; (width * height * 4) as usize];
        let clip = Region::whole(width, height);
        raster_batch(&mut rows(&mut spans), &batch, None, None, clip);
        let mut whole = vec![0u8; (width * height * 4) as usize];
        raster_batch_whole_box(&mut rows(&mut whole), &batch);
        assert!(spans.iter().any(|&byte| byte != 0));
        assert!(spans == whole);
    }

    fn ring_square(x: f32, y: f32, side: f32, colour: [f32; 4]) -> Batch {
        let mut batch = Batch::new(Program::Ring, Texture::None);
        let corner = |dx: f32, dy: f32| {
            TreeVertex::new(x + dx, y + dy, dx / side, dy / side).with_colour(colour)
        };
        batch.vertices = vec![
            corner(0.0, 0.0),
            corner(side, 0.0),
            corner(side, side),
            corner(0.0, 0.0),
            corner(side, side),
            corner(0.0, side),
        ];
        batch
    }

    /// The kept canvas redraws the boxes of the batches that changed, came
    /// or went, and always holds the bytes of a whole redraw.
    #[test]
    fn the_kept_canvas_redraws_the_changed_boxes() {
        let size = (64u32, 48u32);
        let sheets = TreeSheets::default();
        let mut draw = TreeDraw::default();
        draw.batches
            .push(ring_square(0.0, 0.0, 64.0, [0.2, 0.4, 0.6, 0.8]));
        draw.batches
            .push(ring_square(4.0, 4.0, 10.0, [1.0, 0.0, 0.0, 0.5]));
        draw.batches
            .push(ring_square(40.0, 20.0, 12.0, [0.0, 1.0, 0.0, 0.7]));
        let mut cache = CanvasCache::default();
        let first = cache.update(draw.clone(), &sheets, None, None, size);
        assert_eq!(first, vec![Region::whole(size.0, size.1)]);
        assert!(cache.premultiplied() == render(&draw, &sheets, None, size.0, size.1));
        // Nothing changed: nothing is redrawn.
        assert!(
            cache
                .update(draw.clone(), &sheets, None, None, size)
                .is_empty()
        );
        // The middle square moves: its old and new boxes, merged.
        draw.batches[1] = ring_square(6.0, 5.0, 10.0, [1.0, 0.0, 0.0, 0.5]);
        let moved = cache.update(draw.clone(), &sheets, None, None, size);
        assert_eq!(
            moved,
            vec![Region {
                left: 4,
                top: 4,
                right: 16,
                bottom: 15
            }]
        );
        assert!(cache.premultiplied() == render(&draw, &sheets, None, size.0, size.1));
        // A batch comes and the last changes colour: two boxes apart.
        draw.batches[2] = ring_square(40.0, 20.0, 12.0, [0.0, 0.0, 1.0, 0.7]);
        draw.batches
            .push(ring_square(20.0, 30.0, 6.0, [1.0, 1.0, 1.0, 1.0]));
        assert_eq!(
            cache.update(draw.clone(), &sheets, None, None, size).len(),
            2
        );
        assert!(cache.premultiplied() == render(&draw, &sheets, None, size.0, size.1));
        // It goes again.
        draw.batches.pop();
        assert_eq!(
            cache.update(draw.clone(), &sheets, None, None, size).len(),
            1
        );
        assert!(cache.premultiplied() == render(&draw, &sheets, None, size.0, size.1));
        // The backdrop changes: the whole canvas.
        draw.batches[0] = ring_square(0.0, 0.0, 64.0, [0.2, 0.4, 0.6, 0.9]);
        assert_eq!(
            cache.update(draw.clone(), &sheets, None, None, size),
            vec![Region::whole(size.0, size.1)]
        );
        assert!(cache.premultiplied() == render(&draw, &sheets, None, size.0, size.1));
        // A new size draws everything again.
        let wider = (80, 48);
        assert_eq!(
            cache.update(draw.clone(), &sheets, None, None, wider),
            vec![Region::whole(80, 48)]
        );
        assert!(cache.premultiplied() == render(&draw, &sheets, None, 80, 48));
    }

    /// The image node blends straight alpha: each colour is divided by its
    /// alpha, rounded and capped; a transparent pixel is black.
    #[test]
    fn the_canvas_is_handed_over_with_straight_alpha() {
        let canvas = [
            64u8, 32, 0, 128, 200, 10, 255, 255, 9, 9, 9, 0, 255, 0, 0, 100,
        ];
        let mut out = [7u8; 16];
        straighten(
            &canvas,
            &mut out,
            4,
            Region {
                left: 0,
                top: 0,
                right: 3,
                bottom: 0,
            },
        );
        assert_eq!(
            out,
            [
                128, 64, 0, 128, 200, 10, 255, 255, 0, 0, 0, 0, 255, 0, 0, 100
            ]
        );
    }
}
