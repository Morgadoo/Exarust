//! The in-game HUD's power marker and skill-ready alert, drawn.
//!
//! Both indicators show the skill-icon atlas's first cell (the 96-pixel
//! cell at the origin of `sgsklicons.rfi`, which the skill unit adds to its
//! 8-unit atlas before the technique groups) sitting on a soft black halo:
//!
//! - **The halo** is the skin's rounded fill over the two-unit square three
//!   to five units into the icon, grown by eight units each side, its black
//!   at `round(0.6 · pulse · 255)` (the [`crate::skin::rounded_grow`] draw).
//! - **The power marker** (53 units right of the centre, eight above the
//!   bottom) then draws the icon upright through the atlas's plain draw
//!   with the current colour (140, 160, 255) and an alpha byte of
//!   `CVTSS2SI(pulse · 255)`.
//! - **The skill-ready alert** (60 units left of the centre) draws the icon
//!   outlined at the pulse: the face in the skin's seventh label ink, four
//!   one-pixel copies in its third highlight ink, both alphas scaled by the
//!   pulse.
//!
//! The pulse and the shared clock are [`crate::game_gui::StatusClock`]; what
//! shows is decided by the game in [`StatusView`]. The quads go through the
//! interface's draw path ([`crate::gui_draw`]'s programs and blends); each
//! indicator is rasterised into a canvas no larger than its halo, so the
//! per-frame pulse costs a few thousand pixels rather than a window.
//!
//! Rules: `docs/UI.md` (The in-game GUI unit).

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::Game;

use crate::LoadingStatus;
use crate::game_gui::{
    STATUS_GLOW_RADIUS, STATUS_ICON_TINT, StatusClock, StatusDraw, ready_place, status_glow,
    status_glow_alpha, status_icon_alpha, status_place,
};
use crate::gui_draw::{GuiFrame, GuiLayer, GuiTexture, GuiTextures, LayerTexture, icon_batch};
use crate::hud::DrawHud;
use crate::icon_atlas::{IconAtlas, IconImage, IconInks};
use crate::scale::GuiScale;
use crate::skill_gui::{FIRST_ICON, ICON_CELL, ICON_FORMAT, ICON_UNITS, SKILL_ICON_ATLAS};
use crate::skin::{GuiRect, SkinBatch, SkinPalette, rounded_grow};

/// The first cell of the skill-icon atlas image, sized for the atlas: the
/// 96-pixel square at [`FIRST_ICON`] of the stored (bottom-up) rows,
/// resampled to `8 · unit` pixels by the engine's resample-to-size path
/// with the atlas's pixel format. `None` for an image too small to hold
/// the cell or a non-positive unit. The atlas's sharpening of a reduced
/// icon is not applied (no port of that pass exists yet).
pub fn first_skill_icon(width: u16, height: u16, rgba: &[u8], unit: i32) -> Option<IconImage> {
    let side = ICON_UNITS
        .checked_mul(unit)
        .filter(|&s| s > 0 && s <= 0xffff)?;
    let (x, y) = FIRST_ICON;
    if i32::from(width) < x + ICON_CELL || i32::from(height) < y + ICON_CELL {
        return None;
    }
    let count = usize::from(width) * usize::from(height) * 4;
    let image = IconImage {
        width,
        height,
        channels: 4,
        pixels: rgba.get(..count)?.to_vec(),
    };
    let cell = image.region(x, y, ICON_CELL as u16, ICON_CELL as u16);
    if side == ICON_CELL {
        return Some(cell);
    }
    let pixels = exa_render::mipmap::resample_rgba8_to_size(
        &cell.pixels,
        ICON_CELL as usize,
        ICON_CELL as usize,
        side as usize,
        side as usize,
        ICON_FORMAT,
        0,
    );
    (pixels.len() == (side * side * 4) as usize).then_some(IconImage {
        width: side as u16,
        height: side as u16,
        channels: 4,
        pixels,
    })
}

/// The status icon's atlas, its handle and the texture it uploads.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusIcon {
    pub atlas: IconAtlas,
    pub handle: i32,
    pub texture: GuiTexture,
    /// The GUI unit the atlas was built for.
    pub unit: i32,
}

impl StatusIcon {
    /// The 8-unit atlas holding `icon` as its first cell.
    pub fn new(icon: &IconImage, unit: i32) -> Option<Self> {
        let mut atlas = IconAtlas::new(ICON_UNITS * unit, 4);
        atlas.sharpen = crate::skill_gui::icon_sharpened(unit);
        let handle = atlas.add(icon, 0);
        if handle == 0 {
            return None;
        }
        let texture = GuiTexture::from_bottom_up(
            u32::from(atlas.image.width),
            u32::from(atlas.image.height),
            &atlas.image.pixels,
        )?;
        Some(Self {
            atlas,
            handle,
            texture,
            unit,
        })
    }
}

/// The inks the alert's outlined icon reads from the skin's palette: the
/// face is the seventh label ink, the outline the third highlight ink.
pub fn status_inks(palette: &SkinPalette) -> IconInks {
    IconInks {
        face: palette.text[7],
        outline: palette.highlight[2],
        glow: [0; 4],
    }
}

/// The halo layer of an indicator at `place` with the eased pulse.
fn halo(place: (i32, i32), unit: i32, pulse: f32) -> GuiLayer {
    let [x0, y0, x1, y1] = status_glow(place, unit);
    let mut batch = SkinBatch::new();
    rounded_grow(
        &mut batch,
        GuiRect {
            x0: x0 as f32,
            y0: y0 as f32,
            x1: x1 as f32,
            y1: y1 as f32,
        },
        unit,
        STATUS_GLOW_RADIUS,
        status_glow_alpha(pulse),
    );
    GuiLayer::new(batch, LayerTexture::None)
}

/// The layers of the power marker: its halo, then the icon tinted
/// (140, 160, 255) at the pulse's alpha byte.
pub fn marker_layers(icon: &StatusIcon, place: (i32, i32), unit: i32, pulse: f32) -> Vec<GuiLayer> {
    let [r, g, b] = STATUS_ICON_TINT;
    let quads = icon.atlas.draw(icon.handle, place.0 as f32, place.1 as f32);
    vec![
        halo(place, unit, pulse),
        GuiLayer::new(
            icon_batch(&quads, [r, g, b, status_icon_alpha(pulse)]),
            LayerTexture::Owned(icon.texture.clone()),
        ),
    ]
}

/// The layers of the skill-ready alert: its halo, then the icon outlined
/// at the pulse in the skin's inks.
pub fn ready_layers(
    icon: &StatusIcon,
    place: (i32, i32),
    unit: i32,
    pulse: f32,
    inks: &IconInks,
) -> Vec<GuiLayer> {
    let quads = icon
        .atlas
        .draw_outlined(icon.handle, place.0, place.1, pulse, inks);
    vec![
        halo(place, unit, pulse),
        GuiLayer::new(
            icon_batch(&quads, [255; 4]),
            LayerTexture::Owned(icon.texture.clone()),
        ),
    ]
}

/// The two indicators' layers for a window, in the original's order (the
/// marker, then the alert); an indicator that does not draw has none.
pub fn status_layers(
    icon: &StatusIcon,
    draw: StatusDraw,
    window: (u32, i32),
    inks: &IconInks,
) -> [Vec<GuiLayer>; 2] {
    let unit = icon.unit;
    let marker = draw.marker.map_or_else(Vec::new, |pulse| {
        marker_layers(icon, status_place(window.0, window.1, unit), unit, pulse)
    });
    let ready = draw.ready.map_or_else(Vec::new, |pulse| {
        ready_layers(
            icon,
            ready_place(window.0, window.1, unit),
            unit,
            pulse,
            inks,
        )
    });
    [marker, ready]
}

/// A small rasterised piece of the interface: its window-pixel corner,
/// size and straight-alpha sRGB pixels, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCanvas {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Rasterises `layers` through the interface's draw path into the
/// smallest pixel box holding every vertex, the layers moved so the box's
/// corner is the canvas's origin. `None` when there is nothing to draw.
pub fn rasterise_status(layers: &[GuiLayer]) -> Option<StatusCanvas> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for vertex in layers.iter().flat_map(|layer| layer.batch.vertices.iter()) {
        let b = bounds.get_or_insert((vertex.x, vertex.y, vertex.x, vertex.y));
        b.0 = b.0.min(vertex.x);
        b.1 = b.1.min(vertex.y);
        b.2 = b.2.max(vertex.x);
        b.3 = b.3.max(vertex.y);
    }
    let (x0, y0, x1, y1) = bounds?;
    let (left, top) = (x0.floor() as i32, y0.floor() as i32);
    let (width, height) = (
        (x1.ceil() as i32 - left).max(0) as u32,
        (y1.ceil() as i32 - top).max(0) as u32,
    );
    if width == 0 || height == 0 {
        return None;
    }
    let mut frame = GuiFrame::new();
    for layer in layers {
        let mut layer = layer.clone();
        for vertex in &mut layer.batch.vertices {
            vertex.x -= left as f32;
            vertex.y -= top as f32;
        }
        frame.push(layer);
    }
    let rgba = crate::gui_draw::rasterise(&[frame], &GuiTextures::default(), width, height);
    Some(StatusCanvas {
        x: left,
        y: top,
        width,
        height,
        rgba,
    })
}

/// What the game tells the HUD about the indicators each frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusView {
    /// The HUD is switched on (a stamped pulse clock clears when it is off).
    pub enabled: bool,
    /// The view has a player.
    pub player: bool,
    /// The player knows powers and none is a new pick
    /// ([`crate::game_gui::status_marker`]).
    pub marker: bool,
    /// The player's skills call for the alert
    /// ([`crate::game_gui::ready_alert`]).
    pub ready: bool,
}

/// The two indicators sit over the compass and under the bars, as the
/// original's draw order has them.
const STATUS_Z: i32 = 898;

#[derive(Component)]
struct StatusNode(usize);

/// The atlas image, the built icon and what each node last showed.
#[derive(Resource, Default)]
struct StatusState {
    clock: StatusClock,
    /// The atlas image's size and stored rows.
    image: Option<(u16, u16, Vec<u8>)>,
    icon: Option<StatusIcon>,
    shown: [Option<Vec<GuiLayer>>; 2],
    images: [Option<Handle<Image>>; 2],
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<StatusView>()
        .init_resource::<StatusState>()
        .add_systems(Startup, spawn)
        .add_systems(Update, draw_status.in_set(DrawHud));
}

fn spawn(mut commands: Commands, game: Option<Res<Game>>, mut state: ResMut<StatusState>) {
    if let Some(game) = game {
        match game.load_rgba(SKILL_ICON_ATLAS) {
            Ok(image) => {
                if let (Ok(w), Ok(h)) = (u16::try_from(image.width), u16::try_from(image.height)) {
                    state.image = Some((w, h, image.pixels));
                }
            }
            Err(error) => warn!("{SKILL_ICON_ATLAS}: {error}: no HUD status icons"),
        }
    }
    for index in 0..2 {
        commands.spawn((
            Name::new(if index == 0 {
                "HudPowerMarker"
            } else {
                "HudSkillAlert"
            }),
            StatusNode(index),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            ImageNode::default(),
            GlobalZIndex(STATUS_Z),
            Visibility::Hidden,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_status(
    view: Res<StatusView>,
    gui: Res<GuiScale>,
    time: Res<Time<Real>>,
    loading: Option<Res<LoadingStatus>>,
    mut state: ResMut<StatusState>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&StatusNode, &mut Node, &mut ImageNode, &mut Visibility)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    let draw = state.clock.frame(
        view.enabled,
        view.player && loading.is_none(),
        view.marker,
        view.ready,
        now,
    );
    let unit = gui.scale as i32;
    if draw != StatusDraw::default() && state.icon.as_ref().is_none_or(|icon| icon.unit != unit) {
        state.icon = state
            .image
            .as_ref()
            .and_then(|(w, h, rgba)| first_skill_icon(*w, *h, rgba, unit))
            .and_then(|icon| StatusIcon::new(&icon, unit));
    }
    let layers = match state.icon.as_ref() {
        Some(icon) => status_layers(
            icon,
            draw,
            (window.physical_width(), window.physical_height() as i32),
            &status_inks(&crate::line_skin::line_palette()),
        ),
        None => [Vec::new(), Vec::new()],
    };
    let scale = 1.0 / window.scale_factor();
    for (node, mut layout, mut sprite, mut visibility) in &mut nodes {
        let index = node.0.min(1);
        let wanted = &layers[index];
        if wanted.is_empty() {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        }
        if state.shown[index].as_ref() != Some(wanted) {
            let Some(canvas) = rasterise_status(wanted) else {
                continue;
            };
            let mut image = Image::new(
                Extent3d {
                    width: canvas.width,
                    height: canvas.height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                canvas.rgba,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::nearest();
            match state.images[index].clone() {
                Some(handle) if images.contains(&handle) => {
                    if let Some(mut slot) = images.get_mut(&handle) {
                        *slot = image;
                    }
                }
                _ => state.images[index] = Some(images.add(image)),
            }
            layout.left = px(canvas.x as f32 * scale);
            layout.top = px(canvas.y as f32 * scale);
            layout.width = px(canvas.width as f32 * scale);
            layout.height = px(canvas.height as f32 * scale);
            state.shown[index] = Some(wanted.clone());
        }
        if let Some(handle) = &state.images[index]
            && sprite.image != *handle
        {
            sprite.image = handle.clone();
        }
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic 4×4-unit atlas image whose first cell is opaque red and
    /// the rest transparent.
    fn sheet(width: u16, height: u16) -> Vec<u8> {
        let mut rgba = vec![0u8; usize::from(width) * usize::from(height) * 4];
        for y in 0..ICON_CELL as usize {
            for x in 0..ICON_CELL as usize {
                let i = (y * usize::from(width) + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
        rgba
    }

    #[test]
    fn the_first_cell_is_cut_and_sized_for_the_atlas() {
        let rgba = sheet(192, 192);
        // At twelve pixels a unit the cell is already 96 pixels: no resample.
        let icon = first_skill_icon(192, 192, &rgba, 12).unwrap();
        assert_eq!((icon.width, icon.height), (96, 96));
        assert!(icon.pixels.chunks(4).all(|p| p == [255, 0, 0, 255]));
        // At four pixels a unit it is resampled to 32 pixels and stays red.
        let icon = first_skill_icon(192, 192, &rgba, 4).unwrap();
        assert_eq!((icon.width, icon.height), (32, 32));
        let centre = (16 * 32 + 16) * 4;
        assert_eq!(&icon.pixels[centre..centre + 4], &[255, 0, 0, 255]);
        // An image short of the cell, a short buffer or a bad unit: none.
        assert!(first_skill_icon(95, 192, &rgba, 4).is_none());
        assert!(first_skill_icon(192, 192, &rgba[..100], 4).is_none());
        assert!(first_skill_icon(192, 192, &rgba, 0).is_none());
    }

    fn icon(unit: i32) -> StatusIcon {
        let rgba = sheet(96, 96);
        StatusIcon::new(&first_skill_icon(96, 96, &rgba, unit).unwrap(), unit).unwrap()
    }

    #[test]
    fn the_marker_draws_a_halo_then_the_tinted_icon() {
        let icon = icon(4);
        assert_eq!(icon.handle, 1);
        assert_eq!(icon.atlas.cell, 32);
        let layers = marker_layers(&icon, (100, 200), 4, 0.5);
        assert_eq!(layers.len(), 2);
        // The halo: the square 112..120 grown by 8 units (32 px) each side,
        // black at round(0.3 · 255) = 76 (0.6 · 0.5 in double precision).
        let halo = &layers[0].batch;
        let xs: Vec<f32> = halo.vertices.iter().map(|v| v.x).collect();
        assert_eq!(xs, [80.0, 80.0, 152.0, 152.0]);
        assert_eq!(halo.vertices[0].ink, [0, 0, 0, 76]);
        assert_eq!(layers[0].texture, LayerTexture::None);
        // The icon: one 32-pixel quad at the place, (140, 160, 255) at
        // CVTSS2SI(127.5) = 128.
        let quad = &layers[1].batch;
        assert_eq!(quad.vertices.len(), 4);
        assert_eq!((quad.vertices[0].x, quad.vertices[0].y), (100.0, 200.0));
        assert_eq!((quad.vertices[2].x, quad.vertices[2].y), (132.0, 232.0));
        assert!(quad.vertices.iter().all(|v| v.ink == [140, 160, 255, 128]));
    }

    #[test]
    fn the_alert_draws_the_icon_outlined_in_the_skin_inks() {
        let icon = icon(4);
        let palette = crate::line_skin::line_palette();
        let inks = status_inks(&palette);
        assert_eq!(inks.face, palette.text[7]);
        assert_eq!(inks.outline, palette.highlight[2]);
        let layers = ready_layers(&icon, (40, 60), 4, 1.0, &inks);
        // The face, then four outline copies a pixel left, right, above and
        // below.
        let quads = &layers[1].batch;
        assert_eq!(quads.vertices.len(), 20);
        assert_eq!(quads.vertices[0].ink, inks.face);
        assert_eq!(quads.vertices[4].ink, inks.outline);
        assert_eq!(quads.vertices[4].x, 39.0);
        assert_eq!(quads.vertices[8].x, 41.0);
    }

    #[test]
    fn the_indicators_sit_at_their_places_and_rasterise_small() {
        let icon = icon(4);
        let inks = status_inks(&crate::line_skin::line_palette());
        let draw = StatusDraw {
            marker: Some(1.0),
            ready: None,
        };
        let [marker, ready] = status_layers(&icon, draw, (1600, 900), &inks);
        assert!(ready.is_empty());
        // 53 units right of the centre, 8 above the bottom.
        assert_eq!(marker[1].batch.vertices[0].x, 1012.0);
        assert_eq!(marker[1].batch.vertices[0].y, 868.0);
        let canvas = rasterise_status(&marker).unwrap();
        // The halo's box: 5 units left and above the icon, 18 units square.
        assert_eq!((canvas.x, canvas.y), (992, 848));
        assert_eq!((canvas.width, canvas.height), (72, 72));
        // The icon's middle is the red cell under the blue tint: red only,
        // fully covered.
        let at = |x: u32, y: u32| {
            let i = ((y * canvas.width + x) * 4) as usize;
            [
                canvas.rgba[i],
                canvas.rgba[i + 1],
                canvas.rgba[i + 2],
                canvas.rgba[i + 3],
            ]
        };
        let middle = at(36, 36);
        assert_eq!(middle[3], 255);
        assert_eq!(middle[1], 0);
        assert!(middle[0] > 100, "{middle:?}");
        // A corner of the box is outside the halo's rounded cut.
        assert_eq!(at(0, 0)[3], 0);
        assert!(rasterise_status(&[]).is_none());
    }

    /// The installed atlas holds the first cell (skipped without
    /// `EXANIMA_DIR`).
    #[test]
    fn the_installed_status_icon() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let rfi = exa_formats::rfi::Rfi::parse(game.texture(SKILL_ICON_ATLAS).unwrap()).unwrap();
        let rgba = rfi.decode_rgba8().unwrap();
        let (w, h) = (rfi.header.width as u16, rfi.header.height as u16);
        let icon = first_skill_icon(w, h, &rgba, 4).unwrap();
        assert_eq!((icon.width, icon.height), (32, 32));
        assert!(icon.pixels.chunks(4).any(|p| p[3] > 0));
        assert!(StatusIcon::new(&icon, 4).is_some());
    }
}
