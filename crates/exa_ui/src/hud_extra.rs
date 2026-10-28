//! The in-game HUD's extra layers: the compass and the level curtain.
//!
//! The compass is the game unit's `compass` texture drawn as one rotated
//! square at the view's bottom left, its four corners turned by the
//! camera's heading and its texture coordinates mirrored across the
//! vertical axis, all four vertices taking the interface's fade as an
//! opacity of `round((1 − fade) · 192)` of 255. The level curtain is the
//! black page drawn over the whole view while a level ends, its opacity the
//! fall's share of 255.
//!
//! The game sets [`CompassView`] and [`CurtainView`] before [`DrawHud`];
//! this module paints them under the bars, as the original's draw order
//! says. The status marker and the skill-ready alert are [`crate::game_gui`]
//! rules but are not painted here: their icons are skin frames whose atlas
//! the evidence does not name.
//!
//! Rules: `docs/UI.md` (The in-game GUI unit), `docs/RENDERING.md` (The
//! interface).

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::Game;
use exa_formats::rfi::Rfi;

use crate::LoadingStatus;
use crate::blend::{Canvas, linear_opacity};
use crate::game_gui::{compass_alpha, compass_centre, compass_extent};
use crate::hud::DrawHud;
use crate::scale::GuiScale;

/// The compass's texture in the game's resource pack (the `.rfi` suffix is
/// added by the loader).
pub const COMPASS: &str = "compass";

/// The compass's picture: sRGB pixels, stored bottom row first as the
/// texture is.
#[derive(Clone, Debug, PartialEq)]
pub struct CompassTexture {
    /// The texture's size in texels.
    pub width: usize,
    pub height: usize,
    /// Straight-alpha RGBA, 0–255, the stored bottom row first.
    pub rgba: Vec<u8>,
}

impl CompassTexture {
    /// From decoded RGBA pixels, stored bottom row first.
    pub fn new(width: usize, height: usize, rgba: Vec<u8>) -> Option<Self> {
        (width > 0 && height > 0 && rgba.len() == width.checked_mul(height)?.checked_mul(4)?)
            .then_some(Self {
                width,
                height,
                rgba,
            })
    }

    /// From the game's texture file. The decoded rows are the file's, which
    /// run bottom to top as the interface's other textures do.
    pub fn from_rfi(rfi: &Rfi) -> Option<Self> {
        let (width, height) = (rfi.header.width as usize, rfi.header.height as usize);
        Self::new(width, height, rfi.decode_rgba8().ok()?)
    }

    /// The colour under a texture coordinate, both 0–1 with v = 0 the
    /// bottom row, bilinear as the original's linear filter. Coordinates
    /// are clamped to the edge; the answer is sRGB and straight alpha.
    pub fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let (w, h) = (self.width as f32, self.height as f32);
        // Texel centres sit at (i + 0.5) / size; the stored rows run bottom
        // to top, so v = 0 is stored row 0.
        let x = (u * w - 0.5).clamp(0.0, w - 1.0);
        let y = (v * h - 0.5).clamp(0.0, h - 1.0);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let texel = |i: usize, j: usize| {
            let k = (j * self.width + i) * 4;
            let b = &self.rgba[k..k + 4];
            [
                b[0] as f32 / 255.0,
                b[1] as f32 / 255.0,
                b[2] as f32 / 255.0,
                b[3] as f32 / 255.0,
            ]
        };
        let (a, b, c, d) = (texel(x0, y0), texel(x1, y0), texel(x0, y1), texel(x1, y1));
        let mut out = [0.0; 4];
        for k in 0..4 {
            let top = a[k] + (b[k] - a[k]) * fx;
            let bottom = c[k] + (d[k] - c[k]) * fx;
            out[k] = top + (bottom - top) * fy;
        }
        out
    }
}

/// The compass quad's four corners (screen pixels, y down) at a GUI unit, a
/// view height and a heading: the square of half extent [`compass_extent`]
/// around [`compass_centre`], turned clockwise by the heading (the
/// instruction negates the camera's angle before building the matrix).
pub fn compass_corners(unit: i32, view_height: i32, heading: f32) -> [[f32; 2]; 4] {
    let (cx, cy) = compass_centre(unit, view_height);
    let e = compass_extent(unit);
    let (sin, cos) = (-heading).sin_cos();
    [[-e, -e], [e, -e], [e, e], [-e, e]]
        .map(|[x, y]| [cx + x * cos - y * sin, cy + x * sin + y * cos])
}

/// The compass's texture coordinate for a point in the quad's local frame
/// (screen pixels from the compass's centre, before the turn): the corners
/// `(−e,−e)`, `(e,−e)`, `(e,e)`, `(−e,e)` take `(1,1)`, `(0,1)`, `(0,0)`,
/// `(1,0)`, so the picture is mirrored across the vertical axis.
pub fn compass_uv(local: [f32; 2], extent: f32) -> (f32, f32) {
    (
        0.5 - local[0] / (2.0 * extent),
        0.5 - local[1] / (2.0 * extent),
    )
}

/// Paints the compass for a view and a heading at the interface fade. The
/// canvas is the square around the compass; pixels whose centres fall in
/// the turned square take the texture's colour and its coverage times the
/// fade's opacity. No canvas is painted for a fully transparent fade.
pub fn paint_compass(
    texture: &CompassTexture,
    unit: i32,
    view_height: i32,
    heading: f32,
    fade: f32,
) -> Canvas {
    let (cx, cy) = compass_centre(unit, view_height);
    let e = compass_extent(unit);
    let (sin, cos) = heading.sin_cos();
    let alpha = f32::from(compass_alpha(fade)) / 255.0;
    // The turned square's axis-aligned box, which is wider than the square
    // by up to √2 at a diagonal heading.
    let half = e * (sin.abs() + cos.abs());
    let left = (cx - half).floor() as i32;
    let top = (cy - half).floor() as i32;
    let right = (cx + half).ceil() as i32;
    let bottom = (cy + half).ceil() as i32;
    let mut canvas = Canvas::new(
        left,
        top,
        (right - left).max(0) as usize,
        (bottom - top).max(0) as usize,
    );
    if alpha <= 0.0 {
        return canvas;
    }
    for y in top..bottom {
        for x in left..right {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            // The inverse of the clockwise turn.
            let local = [dx * cos - dy * sin, dx * sin + dy * cos];
            if local[0].abs() > e || local[1].abs() > e {
                continue;
            }
            let (u, v) = compass_uv(local, e);
            let texel = texture.sample(u, v);
            canvas.blend(x, y, [texel[0], texel[1], texel[2]], texel[3] * alpha);
        }
    }
    canvas
}

/// The curtain's opacity node colour for a fall's share 0–1: black at the
/// share, remapped so Bevy's linear blend darkens as the original's gamma
/// blend does.
pub fn curtain_colour(fall: f32) -> Color {
    // A NaN share draws nothing, as the original's CVTSS2SI of a NaN gives
    // an indefinite integer whose low byte is 0.
    let alpha = if fall.is_nan() {
        0.0
    } else {
        fall.clamp(0.0, 1.0)
    };
    Color::srgba(0.0, 0.0, 0.0, linear_opacity([0.0; 3], alpha))
}

/// The compass as the game sets it each frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct CompassView {
    /// The view has a player and the player carries a compass.
    pub visible: bool,
    /// The camera's heading (rad).
    pub heading: f32,
    /// The interface's fade, 0 shown and 1 hidden.
    pub fade: f32,
}

/// The level curtain as the game sets it each frame: the fall's share of
/// 255 for the black page, 0 when no curtain is down.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CurtainView(pub u8);

#[derive(Component)]
struct CompassNode;

#[derive(Component)]
struct CurtainNode;

#[derive(Resource)]
struct CompassImage(Option<CompassTexture>);

/// The compass image and what it was painted for.
#[derive(Resource, Default)]
struct CompassState {
    painted: Option<(u32, u32, i32, u32, u8)>,
    image: Option<Handle<Image>>,
    origin: IVec2,
    size: UVec2,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<CompassView>()
        .init_resource::<CurtainView>()
        .init_resource::<CompassState>()
        .add_systems(Startup, spawn)
        .add_systems(Update, (draw_compass, draw_curtain).in_set(DrawHud));
}

/// The compass sits over the heartbeat border and under the bars; the
/// curtain over both, still under the bars as the original's order has it.
const COMPASS_Z: i32 = 891;
const CURTAIN_Z: i32 = 892;

fn spawn(mut commands: Commands, game: Option<Res<Game>>) {
    let texture = game.as_ref().and_then(|g| {
        g.0.texture(COMPASS)
            .and_then(|b| Rfi::parse(b).ok())
            .and_then(|rfi| CompassTexture::from_rfi(&rfi))
    });
    if texture.is_none() {
        warn!("{COMPASS} missing: no compass drawn");
    }
    commands.insert_resource(CompassImage(texture));
    commands.spawn((
        Name::new("Compass"),
        CompassNode,
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::default(),
        GlobalZIndex(COMPASS_Z),
        Visibility::Hidden,
    ));
    commands.spawn((
        Name::new("LevelCurtain"),
        CurtainNode,
        Node {
            position_type: PositionType::Absolute,
            left: px(0.0),
            top: px(0.0),
            width: percent(100.0),
            height: percent(100.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(CURTAIN_Z),
        Visibility::Hidden,
    ));
}

#[allow(clippy::too_many_arguments)]
fn draw_compass(
    compass: Res<CompassView>,
    gui: Res<GuiScale>,
    texture: Option<Res<CompassImage>>,
    loading: Option<Res<LoadingStatus>>,
    mut state: ResMut<CompassState>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&mut Node, &mut ImageNode, &mut Visibility), With<CompassNode>>,
) {
    let (Ok(window), Ok((mut node, mut sprite, mut visibility))) =
        (windows.single(), nodes.single_mut())
    else {
        return;
    };
    let hide = |visibility: &mut Visibility| {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    };
    let Some(texture) = texture.as_ref().and_then(|t| t.0.as_ref()) else {
        hide(&mut visibility);
        return;
    };
    if !compass.visible || loading.is_some() {
        hide(&mut visibility);
        return;
    }
    let size = UVec2::new(window.physical_width(), window.physical_height());
    let alpha = compass_alpha(compass.fade);
    let key = (
        size.x,
        size.y,
        gui.scale as i32,
        compass.heading.to_bits(),
        alpha,
    );
    if state.painted != Some(key) {
        let canvas = paint_compass(
            texture,
            gui.scale as i32,
            size.y as i32,
            compass.heading,
            compass.fade,
        );
        let mut image = Image::new(
            Extent3d {
                width: canvas.width as u32,
                height: canvas.height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            canvas.to_srgba8(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        if let Some(handle) = state.image.clone()
            && let Some(mut slot) = images.get_mut(&handle)
        {
            *slot = image;
        } else {
            state.image = Some(images.add(image));
        }
        state.origin = IVec2::new(canvas.x, canvas.y);
        state.size = UVec2::new(canvas.width as u32, canvas.height as u32);
        state.painted = Some(key);
    }
    if let Some(handle) = &state.image
        && sprite.image != *handle
    {
        sprite.image = handle.clone();
    }
    let unit = 1.0 / window.scale_factor();
    node.left = px(state.origin.x as f32 * unit);
    node.top = px(state.origin.y as f32 * unit);
    node.width = px(state.size.x as f32 * unit);
    node.height = px(state.size.y as f32 * unit);
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

fn draw_curtain(
    curtain: Res<CurtainView>,
    loading: Option<Res<LoadingStatus>>,
    mut nodes: Query<(&mut BackgroundColor, &mut Visibility), With<CurtainNode>>,
) {
    let Ok((mut colour, mut visibility)) = nodes.single_mut() else {
        return;
    };
    if curtain.0 == 0 || loading.is_some() {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    colour.0 = curtain_colour(f32::from(curtain.0) / 255.0);
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compass_quad_sits_at_the_bottom_left_turned_by_the_heading() {
        // Unit 4 in a 900-pixel view: centre (56, 844), half extent 48.
        assert_eq!(
            compass_corners(4, 900, 0.0),
            [[8.0, 796.0], [104.0, 796.0], [104.0, 892.0], [8.0, 892.0]]
        );
        // A quarter turn clockwise (the heading's sign is negated): the
        // up-left corner lands at the bottom-left of the centre.
        let turned = compass_corners(4, 900, std::f32::consts::FRAC_PI_2);
        let want = [[8.0, 892.0], [8.0, 796.0], [104.0, 796.0], [104.0, 892.0]];
        for (got, want) in turned.iter().zip(want) {
            assert!((got[0] - want[0]).abs() < 1e-3, "{got:?}");
            assert!((got[1] - want[1]).abs() < 1e-3, "{got:?}");
        }
    }

    /// The corners' texture coordinates are mirrored across the vertical
    /// axis: the left corners take u 1, the right ones u 0; the top v 1 and
    /// the bottom v 0.
    #[test]
    fn the_compass_texture_is_mirrored() {
        assert_eq!(compass_uv([-48.0, -48.0], 48.0), (1.0, 1.0));
        assert_eq!(compass_uv([48.0, -48.0], 48.0), (0.0, 1.0));
        assert_eq!(compass_uv([48.0, 48.0], 48.0), (0.0, 0.0));
        assert_eq!(compass_uv([-48.0, 48.0], 48.0), (1.0, 0.0));
        assert_eq!(compass_uv([0.0, 0.0], 48.0), (0.5, 0.5));
    }

    /// A 2 × 2 texture, stored bottom row first: bottom-left red, bottom-
    /// right green, top-left blue, top-right white, all opaque.
    fn texture() -> CompassTexture {
        CompassTexture::new(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, // the stored bottom row
                0, 0, 255, 255, 255, 255, 255, 255, // the top row
            ],
        )
        .unwrap()
    }

    #[test]
    fn sampling_clamps_and_bilinear_blends() {
        let t = texture();
        // v = 0 is the bottom row, v = 1 the top.
        assert_eq!(t.sample(0.0, 0.0), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(t.sample(1.0, 0.0), [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(t.sample(0.0, 1.0), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(t.sample(1.0, 1.0), [1.0, 1.0, 1.0, 1.0]);
        // Texel centres: u = 0.25 is the first column, 0.75 the second.
        assert_eq!(t.sample(0.25, 0.25), [1.0, 0.0, 0.0, 1.0]);
        // Between all four: the four-way mean.
        let mid = t.sample(0.5, 0.5);
        for (k, want) in mid.iter().zip([0.5, 0.5, 0.5, 1.0]) {
            assert!((k - want).abs() < 1e-6, "{mid:?}");
        }
        // Clamped at the edges.
        assert_eq!(t.sample(-1.0, -1.0), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(t.sample(2.0, 2.0), [1.0, 1.0, 1.0, 1.0]);
        assert!(CompassTexture::new(0, 2, vec![]).is_none());
        assert!(CompassTexture::new(2, 2, vec![0; 15]).is_none());
    }

    #[test]
    fn the_compass_is_painted_at_its_place_and_fade() {
        // Unit 4: the compass centre is (56, 844), its square 96 wide.
        let c = paint_compass(&texture(), 4, 900, 0.0, 0.0);
        assert_eq!((c.x, c.y, c.width, c.height), (8, 796, 96, 96));
        // The centre: the middle of the texture (half of each corner).
        let mid = c.get(56, 844).unwrap();
        assert!((mid[0] - 0.5).abs() < 0.02, "{mid:?}");
        assert!((mid[1] - 0.5).abs() < 0.02, "{mid:?}");
        // The fade 1 draws nothing.
        let none = paint_compass(&texture(), 4, 900, 0.0, 1.0);
        assert!(none.pixels.iter().all(|p| p[3] == 0.0));
        // Half faded: 96 of 255 over an opaque texture.
        let half = paint_compass(&texture(), 4, 900, 0.0, 0.5);
        let p = half.get(56, 844).unwrap();
        assert!((p[3] - 96.0 / 255.0).abs() < 0.03, "{p:?}");
        // Outside the square nothing is drawn.
        assert_eq!(c.get(7, 844), None);
        assert_eq!(c.get(56, 892), None);
    }

    #[test]
    fn the_compass_turn_moves_the_picture() {
        // Turned a quarter clockwise, the top-left corner of the picture
        // lands at the square's top-right: the texture's red is there.
        let c = paint_compass(&texture(), 4, 900, std::f32::consts::FRAC_PI_2, 0.0);
        let top_right = c.get(100, 800).unwrap();
        // The turned picture's corners: red is the local up-left corner,
        // now at the screen's up-right; sample near it.
        assert!(top_right[0] > 0.5 || top_right[1] > 0.5, "{top_right:?}");
        // The centre still averages the same four texels, at the fade's
        // 192 of 255.
        let mid = c.get(56, 844).unwrap();
        assert!((mid[3] - 192.0 / 255.0).abs() < 0.02, "{mid:?}");
        // At a diagonal heading the turned square reaches √2 times as far:
        // its corner is painted, not clipped.
        let c = paint_compass(&texture(), 4, 900, std::f32::consts::FRAC_PI_4, 0.0);
        assert_eq!((c.x, c.y, c.width, c.height), (-12, 776, 136, 136));
        let corner = c.get(56, 777).unwrap();
        assert!(corner[3] > 0.0, "{corner:?}");
    }

    #[test]
    fn the_curtain_darkens_by_its_share() {
        assert_eq!(curtain_colour(0.0).alpha(), 0.0);
        let full = curtain_colour(1.0);
        assert_eq!(full.alpha(), 1.0);
        let half = curtain_colour(0.5).alpha();
        // Black at half opacity in gamma space: Bevy's linear blend needs
        // a higher opacity (about 0.67) to darken the floor as much.
        assert!((half - 0.67).abs() < 0.01, "{half}");
        // A share past either end clamps.
        assert_eq!(curtain_colour(-1.0).alpha(), 0.0);
        assert_eq!(curtain_colour(2.0).alpha(), 1.0);
        assert_eq!(curtain_colour(f32::NAN).alpha(), 0.0);
    }

    /// The installed compass texture is square and has painted texels
    /// (skipped without `EXANIMA_DIR`).
    #[test]
    fn the_installed_compass_texture() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let rfi = Rfi::parse(game.texture(COMPASS).unwrap()).unwrap();
        assert_eq!((rfi.header.width, rfi.header.height), (256, 256));
        let tex = CompassTexture::from_rfi(&rfi).unwrap();
        let inked = tex
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .count();
        assert!(inked > tex.width * tex.height / 20, "{inked}");
    }
}
