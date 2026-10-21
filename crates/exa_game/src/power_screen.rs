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
use crate::player::PlayerBody;
use crate::thaum::casting::{CastBody, code};
use crate::thaum::displacement::{DisplacementAim, DisplacementStart, displacement_front_end};
use crate::thaum::energy::energy_front_end;
use crate::thaum::force::{ForceStart, ForceWorld, force_front_end};
use crate::thaum::tree::{
    DOMAIN_DISPLACEMENT, DOMAIN_ENERGY, DOMAIN_FORCE, PowerBar, PowerRecord, TreeMode, TreeScreen,
};
use crate::thaum::visuals::{self, CreatorInput, EffectKind, VisualSpawn};

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
            .init_resource::<PowerBody>()
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

/// The body a bar cast is priced and gated on.
#[derive(Resource, Debug, Clone, Default)]
pub struct PowerBody {
    pub body: crate::thaum::casting::CastBody,
}

fn load_screen(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut screen: ResMut<PowerScreen>,
    mut body: ResMut<PowerBody>,
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
    body.body.focus = 1.0;
    commands.insert_resource(AssetsLoaded);
}

fn hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Option<Res<Bindings>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&GlobalTransform, With<PlayerBody>>,
    mut screen: ResMut<PowerScreen>,
    mut body: ResMut<PowerBody>,
    mut spawns: MessageWriter<VisualSpawn>,
) {
    let Some(bindings) = bindings else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as f32;
    let height = window.physical_height() as f32;
    let PowerScreen {
        open,
        bar_open,
        tree,
        record,
        bar,
        texts,
        domain_files,
        dirty,
        ..
    } = &mut *screen;
    if bindings
        .key_code(Action::PowerTree)
        .is_some_and(|key| keys.just_pressed(key))
    {
        if *open {
            tree.leave(bar, record, 0);
            *open = false;
            *bar_open = false;
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
            bar.load(record);
        }
        *dirty = true;
        return;
    }
    if bindings
        .key_code(Action::PowerBar)
        .is_some_and(|key| keys.just_pressed(key))
    {
        if *bar_open {
            bar.store(record, 0);
            *bar_open = false;
        } else {
            bar.load(record);
            *bar_open = true;
        }
        *dirty = true;
    }
    if !*bar_open {
        return;
    }
    let using = bindings
        .key_code(Action::UsePower)
        .is_some_and(|key| keys.just_pressed(key));
    if using {
        let now = 0;
        let mut force_start = None;
        let mut displacement_start = None;
        let (domain_body, domain_aim) = (ForceWorld::default(), DisplacementAim::default());
        let result = tree.cast_slot(
            record,
            bar,
            None,
            &mut body.body,
            now,
            |domain, asked, cast: &mut CastBody| match domain {
                DOMAIN_FORCE => {
                    let (result, start) = force_front_end(asked, cast, &domain_body);
                    force_start = start;
                    result
                }
                DOMAIN_DISPLACEMENT => {
                    let (result, start) = displacement_front_end(asked, cast, &domain_aim);
                    displacement_start = start;
                    result
                }
                DOMAIN_ENERGY => energy_front_end(asked, cast).0,
                _ => code::REFUSED,
            },
        );
        let place = players.single().ok();
        let at = place.map(|t| t.translation()).unwrap_or(Vec3::ZERO);
        let heading = place.map(|t| t.forward().as_vec3()).unwrap_or(Vec3::NEG_Z);
        let input = CreatorInput {
            body: at,
            heading,
            hand: at + heading * 40.0 + Vec3::Y * 120.0,
            point: None,
            roll: 0.5,
        };
        if result.result > 0 {
            if let Some(start) = force_start {
                let kind = match start {
                    ForceStart::Blast { setup, .. } => match setup.power {
                        0x202 => Some(EffectKind::Bolt),
                        0x204 => Some(EffectKind::Wave),
                        _ => Some(EffectKind::Blast),
                    },
                    ForceStart::Torrent { .. } => Some(EffectKind::Torrent),
                    ForceStart::Barrier { .. } => Some(EffectKind::Barrier),
                    ForceStart::Bubble { .. } => Some(EffectKind::Bubble),
                    ForceStart::Barricade { .. } => Some(EffectKind::Barricade),
                    ForceStart::Shell { .. } => Some(EffectKind::Shell),
                    ForceStart::Kinesis { .. } => Some(EffectKind::Kinesis),
                    ForceStart::Throw { .. } => Some(EffectKind::Throw),
                    ForceStart::Strikes { .. } => Some(EffectKind::Strikes),
                    ForceStart::ThrowMissed => None,
                };
                if let Some(kind) = kind {
                    spawns.write(VisualSpawn(visuals::creator(kind, &input)));
                }
            }
            if let Some(start) = displacement_start {
                let kind = match start {
                    DisplacementStart::Vanish { .. } => Some(EffectKind::Vanish),
                    DisplacementStart::LongDoor { .. } => Some(EffectKind::LongDoor),
                    DisplacementStart::Constant { .. } => Some(EffectKind::Constant),
                    DisplacementStart::Summon { .. } => Some(EffectKind::Summon),
                    DisplacementStart::EndVanish => None,
                };
                if let Some(kind) = kind {
                    spawns.write(VisualSpawn(visuals::creator(kind, &input)));
                }
            }
        }
        *dirty = true;
        return;
    }
    for (index, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
    ]
    .iter()
    .enumerate()
    {
        if keys.just_pressed(*key) && bar.select(index) {
            *dirty = true;
        }
    }
    if keys.just_pressed(KeyCode::ArrowLeft) && bar.cycle(-1, Some(record)) {
        *dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowRight) && bar.cycle(1, Some(record)) {
        *dirty = true;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if *open {
            tree.leave(bar, record, 0);
            *open = false;
        }
        *bar_open = false;
        *dirty = true;
    }
    let _ = body;
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
    let hidden = !*open;
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
    if !screen.open && !screen.bar_open {
        for (_, _, mut visibility, _) in &mut canvas {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let now = time.elapsed().as_millis();
    if (screen.open || screen.bar_open) && screen.tree.settle() {
        screen.dirty = true;
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
    let view = tree.view(bar, width as f32, height as f32, scale, now as u32);
    let draw = if tree_open {
        // The tree pass already ends with the bar.
        let mut draw = tree_draw::tree_draw(&view);
        // The tooltip: the hovered node's name and wrapped description.
        if tree.node_under > 0 && (tree.node_under as usize) <= tree.nodes.len() {
            let (name, description) = tree.node_text(tree.node_under as usize - 1, texts);
            let wrapped = wrap_text(description, 59);
            let tooltip = tree_draw::tooltip_draw(&view, &String::from_utf8_lossy(name), &wrapped);
            draw.batches.extend(tooltip.batches);
            draw.texts.extend(tooltip.texts);
        }
        if let Some(dragged) = drag.clone() {
            draw.batches.extend(
                tree_draw::preview_draw(
                    dragged.power,
                    dragged.flags,
                    dragged.at[0],
                    dragged.at[1],
                    scale * 6.0,
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
    for batch in &draw.batches {
        let texture = texture_of(batch.texture, sheets);
        raster_batch(&mut canvas, width, height, batch, texture);
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
        Texture::Domain => sheets.domains.get(1).and_then(Option::as_ref),
        Texture::Frames => sheets.frames.as_ref(),
        Texture::Mods => sheets.mods.as_ref(),
        Texture::Flamelets => sheets.flamelets.as_ref(),
        Texture::Wheel => sheets.wheel.as_ref(),
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
                let [r, g, bl, alpha] = shade(batch, vertex, texture);
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
fn shade(batch: &Batch, vertex: TreeVertex, texture: Option<&RgbaImage>) -> [f32; 4] {
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
            let mut ty = vertex.v;
            let b = vertex.u2;
            let a = 0.5 - b;
            let a = 0.25 - a * a;
            ty -= a * 0.025;
            ty += a * uniforms.par0[2] + uniforms.par0[1];
            let tx = vertex.u;
            let lower = clamp01(ty * 16.0);
            let upper = clamp01(4.0 - ty * 16.0);
            let band = lower * upper;
            let tex = sample(texture, tx, ty)[0];
            let lod = sample(texture, tx, ty)[0];
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

    #[test]
    fn the_ring_shader_draws_a_dot() {
        let mut batch = Batch::new(Program::Ring, Texture::None);
        batch.vertices = vec![TreeVertex::new(0.0, 0.0, 0.5, 0.5)];
        let centre = shade(&batch, batch.vertices[0], None);
        assert!(centre[3] > 0.0);
        let edge = TreeVertex::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(shade(&batch, edge, None)[3], 0.0);
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
        let middle = shade(&batch, centre, None);
        assert!(middle[3] > 0.9);
        let edge = TreeVertex::new(0.0, 0.0, 0.0, 0.0).with_colour([1.0, 1.0, 1.0, 1.0]);
        assert!(shade(&batch, edge, None)[3] < middle[3]);
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
