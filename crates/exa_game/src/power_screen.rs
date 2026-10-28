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
//! The tooltip is the shown choice's name caption (at once) and its wrapped
//! description box (after 600 ms), laid out at the original's places with
//! the widths and line step of the GUI main font at the tree's unit; while
//! a power is dragged every node wears its name instead. Not modelled: the
//! link programs' ribbon texture (the draw treats it as white, so links
//! lack the spline's texture), the GUI
//! skin's own text metrics and button highlight, and the base form's key
//! handlers. The bar's own keys (pick, step, use) belong to the hotkey
//! handler and the cast plugin ([`crate::thaum::cast`]), which keeps the
//! caster's record and this screen's in step.

use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::{Game, RgbaImage, TreeSheets};
use exa_formats::rfi::Rfi;
use exa_ui::font::{BASE_FONT, GameFont};
use exa_ui::gui_core::{FormHost, FormId, GuiRect, GuiTask};
use exa_ui::tree_draw::{self, Batch, Program, TextRun, Texture, TreeDraw, TreeVertex};

use crate::bindings::{Action, Bindings};
use crate::thaum::tree::{BAR_CLOSING, PowerBar, PowerRecord, TreeMode, TreeScreen, TreeSound};

/// The time between redraws while the screen animates.
const REDRAW_MS: u128 = 33;
/// The bar's canvas height in GUI units.
const BAR_UNITS: f32 = 20.0;

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
    pub font: Option<GameFont>,
    /// The sound the tree's last action names, for the audio path: showing
    /// it plays `ThaumShow02`; a hidden screen is silent. The running game
    /// has no UI-sound playback yet.
    pub sound: Option<TreeSound>,
    /// A redraw is needed.
    pub dirty: bool,
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
            font: None,
            sound: None,
            dirty: true,
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
            .add_systems(Update, (hotkeys, pointer, draw).chain());
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

fn load_screen(mut commands: Commands, game: Option<Res<Game>>, mut screen: ResMut<PowerScreen>) {
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
        screen.texts = Vec::new();
    }
    commands.insert_resource(AssetsLoaded);
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
    if bindings
        .key_code(Action::PowerTree)
        .is_some_and(|key| keys.just_pressed(key))
    {
        if *open {
            // Closing stores the bar and the domain, then fades out over
            // 256 ms; the draw turns the screen off at the fade's end.
            tree.leave(bar, record);
            tree.hide(now);
            tree.close_bar(now);
        } else {
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
        }
        *dirty = true;
        return;
    }
    // The bar's own keys are the hotkey handler's (the cast plugin); the
    // tree keeps its screen and its Escape.
    if keys.just_pressed(KeyCode::Escape) {
        if *open {
            tree.leave(bar, record);
            tree.hide(now);
            tree.close_bar(now);
        } else {
            *bar_open = false;
        }
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
        ..
    } = &mut *screen;
    let hidden = !*open || tree.closing();
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
        let _ = tree.press(input, Some(record), bar);
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
            bar.drop(x, y, width, &power);
        } else {
            let _ = tree.press(input, Some(record), bar);
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
    let mut steps = 0.0;
    for event in wheel.read() {
        steps += event.y;
    }
    if steps != 0.0 {
        tree.zoom_by(steps, now);
        *dirty = true;
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
    let scale = window.scale_factor() * 4.0;
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
    let view = tree.view(bar, width as f32, height as f32, scale, now as u32);
    let draw = if tree_open {
        // The tree pass already ends with the bar.
        let mut draw = tree_draw::tree_draw(&view);
        // The tooltip: the shown choice's name caption and its wrapped
        // description box, measured with the GUI font's advances (the
        // glyphs are the running font's).
        if !view.labels && view.hovered > 0 && (view.hovered as usize) <= tree.nodes.len() {
            let (name, description) = tree.node_text(view.hovered as usize - 1, texts);
            let lines = tree_draw::text_split(description, tree_draw::TIP_TEXT_WIDTH);
            let tooltip = tree_draw::tooltip_draw(
                &view,
                &lines,
                &String::from_utf8_lossy(name),
                tree_draw::text_step(scale),
                tree_draw::text_width(name, scale),
            );
            draw.batches.extend(tooltip.batches);
            draw.texts.extend(tooltip.texts);
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
                    tree_draw::text_width(text, scale),
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
                    tree.domain,
                )
                .batches,
            );
        }
        draw
    } else {
        tree_draw::bar_draw(&view)
    };
    let pixels = render(
        &draw,
        &screen.sheets,
        screen.font.as_ref(),
        width,
        canvas_height,
    );
    let image = Image::new(
        Extent3d {
            width,
            height: canvas_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);
    let ui_scale = window.scale_factor();
    let mut spawned = false;
    for (mut image_node, mut node, mut visibility, mut state) in &mut canvas {
        image_node.image = handle.clone();
        node.width = Val::Px(width as f32 / ui_scale);
        node.height = Val::Px(canvas_height as f32 / ui_scale);
        node.left = Val::Px(0.0);
        node.top = Val::Px(0.0);
        *visibility = Visibility::Inherited;
        state.size = (width, canvas_height);
        state.open = screen.open || screen.bar_open;
        spawned = true;
    }
    if !spawned {
        commands.spawn((
            PowerCanvas,
            ImageNode::new(handle),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(width as f32 / ui_scale),
                height: Val::Px(canvas_height as f32 / ui_scale),
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..Node::default()
            },
            CanvasState {
                size: (width, canvas_height),
                open: true,
            },
        ));
    }
    screen.last_draw = now;
    screen.dirty = false;
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

/// The canvas rasteriser: every batch's triangles with the program's
/// fragment arithmetic, then the texts.
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
    for batch in &draw.batches {
        let texture = texture_of(batch.texture, sheets);
        let level = if batch.texture == Texture::Frames {
            coarse.as_ref()
        } else {
            None
        };
        raster_batch(&mut canvas, width, height, batch, texture, level);
    }
    if let Some(font) = font {
        for run in &draw.texts {
            blit_text(&mut canvas, width, height, run, font);
        }
    }
    canvas
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
        let x = x.clamp(0, image.width as i32 - 1) as usize;
        let y = y.clamp(0, image.height as i32 - 1) as usize;
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

/// One batch's triangles.
fn raster_batch(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    batch: &Batch,
    texture: Option<&RgbaImage>,
    coarse: Option<&RgbaImage>,
) {
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
                let [r, g, bl, alpha] = shade(batch, vertex, texture, coarse);
                blend(canvas, width, height, x, y, [r, g, bl, alpha]);
            }
        }
    }
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
            // The link's texture is the spline's white gradient.
            let t = [vertex.u, vertex.v];
            let b = vertex.u2;
            let a0 = 0.5 - b;
            let a0 = 0.25 - a0 * a0;
            let bb = t[1] + (t[1] * 0.05 + uniforms.par0[2] * a0) * 2.0;
            let ty = t[1] * uniforms.par0[0];
            let c = if batch.program == Program::Link {
                let mut a = 1.0 - bb;
                a *= a;
                a *= a;
                a *= a;
                let mut c = 1.0 + a;
                c *= 1.0 * 0.75;
                c
            } else {
                let a = (1.0 - bb).powi(2);
                let c = 1.0 + a;
                let c = c * 1.0 * 0.75;
                c + (a - c) * 0.75
            };
            let _ = ty;
            let a = clamp01(bb * 32.0);
            let b2 = clamp01(c - (bb * bb + uniforms.par0[3]));
            let alpha = a * b2 * b2;
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
    };
    out.iter_mut().for_each(|v| *v = clamp01(*v));
    out
}

fn blend(canvas: &mut [u8], width: u32, height: u32, x: i32, y: i32, colour: [f32; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let a = colour[3];
    if a <= 0.0 {
        return;
    }
    let at = (y as usize * width as usize + x as usize) * 4;
    for (index, channel) in colour.iter().take(3).enumerate() {
        let src = channel * 255.0;
        let dst = f32::from(canvas[at + index]);
        canvas[at + index] = (src * a + dst * (1.0 - a)).clamp(0.0, 255.0) as u8;
    }
    let dst_a = f32::from(canvas[at + 3]) / 255.0;
    canvas[at + 3] = ((a + dst_a * (1.0 - a)) * 255.0).clamp(0.0, 255.0) as u8;
}

/// Blits one text run with the game font, tinted.
fn blit_text(canvas: &mut [u8], width: u32, height: u32, run: &TextRun, font: &GameFont) {
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
                    blend(canvas, width, height, px + ox, py + oy, colour);
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
}
