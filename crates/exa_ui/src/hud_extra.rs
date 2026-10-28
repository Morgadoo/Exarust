//! The in-game HUD's compass.
//!
//! The compass is the game unit's `compass` texture drawn as one turned
//! square at the view's bottom left. The original builds a frame from the
//! camera's backward axis (the third row of the camera's frame) laid flat,
//! its x negated and its y zeroed, then normalised: that axis `d` and its
//! cross product with up give the square's two axes, and the frame's
//! origin is the compass's centre moved three pixels against `d`. The
//! square's corners `(∓e, ∓e)` with texture coordinates `(1,1)`, `(0,1)`,
//! `(0,0)`, `(1,0)` go through that frame, which reflects them, so the
//! picture shows upright and unmirrored, turned clockwise on screen by the
//! camera's yaw. All four vertices take the interface's fade as an opacity
//! of `round((1 − fade) · 192)` of 255.
//!
//! The level curtain the same draw order names is the black layer of
//! [`crate::fade`] (three seconds to full, between the heartbeat and the
//! bars), already driven by the game.
//!
//! The game sets [`CompassView`] before [`DrawHud`]; this module paints it
//! under the bars, as the original's draw order says. The status marker and
//! the skill-ready alert that follow it are [`crate::hud_status`]'s.
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
use crate::blend::Canvas;
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

/// The compass's turning axis `d` from the camera's backward axis laid
/// flat (its x and z): `(−x, 0, z)` scaled by the processor's approximate
/// reciprocal square root of its squared length (summed `(x² + z²) + 0`),
/// zero for a zero axis, as the engine's normalise does.
pub fn compass_axis(back: [f32; 2]) -> [f32; 2] {
    let (x, z) = (-back[0], back[1]);
    let length = (x * x + z * z) + 0.0;
    let scale = if length == 0.0 {
        0.0
    } else {
        exa_physics::engine_math::reciprocal_sqrt(length)
    };
    [x * scale, z * scale]
}

/// The compass frame's origin (screen pixels, y down): [`compass_centre`]
/// plus the axis times −3, as the instruction moves the frame's last row.
pub fn compass_origin(unit: i32, view_height: i32, axis: [f32; 2]) -> [f32; 2] {
    let (cx, cy) = compass_centre(unit, view_height);
    [cx + axis[0] * -3.0, cy + axis[1] * -3.0]
}

/// A point of the quad's local frame `(x, z)` on screen: the frame's first
/// row is `d × up = (−d_z, 0, d_x)`, its third `d`, so the screen point is
/// `x·(−d_z) + z·d_x + origin_x` across and `x·d_x + z·d_z + origin_y`
/// down, summed as the engine's point transform does.
pub fn compass_point(local: [f32; 2], axis: [f32; 2], origin: [f32; 2]) -> [f32; 2] {
    let [x, z] = local;
    let [dx, dz] = axis;
    [
        (x * -dz + 0.0) + (z * dx + origin[0]),
        (x * dx + 0.0) + (z * dz + origin[1]),
    ]
}

/// The compass quad's four corners (screen pixels, y down) at a GUI unit, a
/// view height and the camera's flat backward axis `back` (x, z): the
/// local corners `(−e,−e)`, `(e,−e)`, `(e,e)`, `(−e,e)` through the frame
/// of [`compass_axis`] and [`compass_origin`].
pub fn compass_corners(unit: i32, view_height: i32, back: [f32; 2]) -> [[f32; 2]; 4] {
    let axis = compass_axis(back);
    let origin = compass_origin(unit, view_height, axis);
    let e = compass_extent(unit);
    [[-e, -e], [e, -e], [e, e], [-e, e]].map(|local| compass_point(local, axis, origin))
}

/// The compass's texture coordinate for a point in the quad's local frame
/// (before the frame turns and reflects it): the corners `(−e,−e)`,
/// `(e,−e)`, `(e,e)`, `(−e,e)` take `(1,1)`, `(0,1)`, `(0,0)`, `(1,0)`.
/// The frame's reflection undoes the mirror, so on screen u runs left to
/// right and v bottom to top.
pub fn compass_uv(local: [f32; 2], extent: f32) -> (f32, f32) {
    (
        0.5 - local[0] / (2.0 * extent),
        0.5 - local[1] / (2.0 * extent),
    )
}

/// Paints the compass for a view and the camera's flat backward axis at
/// the interface fade. The canvas is the box around the turned square;
/// pixels whose centres fall in the square take the texture's colour and
/// its coverage times the fade's opacity. No canvas is painted for a fully
/// transparent fade. (The original rasterises the quad on the GPU; the
/// canvas is Exarust's way to draw it.)
pub fn paint_compass(
    texture: &CompassTexture,
    unit: i32,
    view_height: i32,
    back: [f32; 2],
    fade: f32,
) -> Canvas {
    let axis = compass_axis(back);
    let [cx, cy] = compass_origin(unit, view_height, axis);
    let e = compass_extent(unit);
    let [dx, dz] = axis;
    let alpha = f32::from(compass_alpha(fade)) / 255.0;
    // The turned square's axis-aligned box, which is wider than the square
    // by up to √2 at a diagonal heading.
    let half = e * (dx.abs() + dz.abs());
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
            let x_off = x as f32 + 0.5 - cx;
            let y_off = y as f32 + 0.5 - cy;
            // The frame's matrix [[−d_z, d_x], [d_x, d_z]] is symmetric
            // and orthogonal: its own inverse.
            let local = [x_off * -dz + y_off * dx, x_off * dx + y_off * dz];
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

/// The compass as the game sets it each frame.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CompassView {
    /// The view has a player and the player carries a compass.
    pub visible: bool,
    /// The camera's backward axis (the third row of its frame), its x and
    /// z: what the compass turns by.
    pub back: [f32; 2],
    /// The interface's fade, 0 shown and 1 hidden.
    pub fade: f32,
}

impl Default for CompassView {
    /// Hidden, with a camera looking down −z.
    fn default() -> Self {
        Self {
            visible: false,
            back: [0.0, 1.0],
            fade: 0.0,
        }
    }
}

#[derive(Component)]
struct CompassNode;

#[derive(Resource)]
struct CompassImage(Option<CompassTexture>);

/// The compass image and what it was painted for.
#[derive(Resource, Default)]
struct CompassState {
    painted: Option<(u32, u32, i32, u32, u32, u8)>,
    image: Option<Handle<Image>>,
    origin: IVec2,
    size: UVec2,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<CompassView>()
        .init_resource::<CompassState>()
        .add_systems(Startup, spawn)
        .add_systems(Update, draw_compass.in_set(DrawHud));
}

/// The compass sits over the heartbeat border and the curtain and under the
/// bars, as the original's order has it.
const COMPASS_Z: i32 = 896;

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
        compass.back[0].to_bits(),
        compass.back[1].to_bits(),
        alpha,
    );
    if state.painted != Some(key) {
        let canvas = paint_compass(
            texture,
            gui.scale as i32,
            size.y as i32,
            compass.back,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn near(got: [f32; 2], want: [f32; 2]) {
        assert!(
            (got[0] - want[0]).abs() < 0.05 && (got[1] - want[1]).abs() < 0.05,
            "{got:?} vs {want:?}"
        );
    }

    #[test]
    fn the_compass_axis_is_the_flat_backward_axis_with_x_negated() {
        // The processor's reciprocal root of 1 is a little under 1.
        let k = exa_physics::engine_math::reciprocal_sqrt(1.0);
        assert_eq!(compass_axis([0.0, 1.0]), [-0.0, k]);
        assert_eq!(compass_axis([1.0, 0.0]), [-k, 0.0]);
        // A pitched camera's shorter flat axis still comes out unit long.
        let [x, z] = compass_axis([0.3, 0.4]);
        assert!(((x * x + z * z).sqrt() - 1.0).abs() < 2e-3);
        assert!(x < 0.0 && z > 0.0);
        // A camera looking straight down: no axis, the square collapses.
        assert_eq!(compass_axis([0.0, 0.0]), [0.0, 0.0]);
    }

    #[test]
    fn the_compass_quad_sits_at_the_bottom_left_turned_by_the_camera() {
        // Unit 4 in a 900-pixel view: centre (56, 844), half extent 48,
        // the frame's origin three pixels against the axis.
        let corners = compass_corners(4, 900, [0.0, 1.0]);
        // Looking down −z: the local corner (−e, −e) (texture (1, 1)) is
        // the screen's top right, so the picture is upright, unmirrored.
        near(corners[0], [104.0, 793.0]);
        near(corners[1], [8.0, 793.0]);
        near(corners[2], [8.0, 889.0]);
        near(corners[3], [104.0, 889.0]);
        // A quarter turn of the camera's yaw (backward +x): the picture
        // turns a quarter clockwise; the origin moves 3 px right.
        let corners = compass_corners(4, 900, [1.0, 0.0]);
        near(corners[0], [107.0, 892.0]);
        near(corners[1], [107.0, 796.0]);
        near(corners[2], [11.0, 796.0]);
        near(corners[3], [11.0, 892.0]);
    }

    /// The corners' texture coordinates in the quad's own frame, which the
    /// camera frame reflects on screen.
    #[test]
    fn the_compass_texture_coordinates_by_corner() {
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
        // Unit 4: the frame's origin is (56, ≈841), the square 96 wide.
        let c = paint_compass(&texture(), 4, 900, [0.0, 1.0], 0.0);
        assert_eq!((c.x, c.y, c.width, c.height), (8, 793, 96, 96));
        // Upright and unmirrored: blue top left, white top right, red
        // bottom left, green bottom right.
        let at = |x, y| c.get(x, y).unwrap();
        assert_eq!(&at(9, 794)[..3], &[0.0, 0.0, 1.0]);
        assert_eq!(&at(102, 794)[..3], &[1.0, 1.0, 1.0]);
        assert_eq!(&at(9, 887)[..3], &[1.0, 0.0, 0.0]);
        assert_eq!(&at(102, 887)[..3], &[0.0, 1.0, 0.0]);
        // The fade's 192 of 255 over an opaque texture.
        assert!((at(56, 841)[3] - 192.0 / 255.0).abs() < 0.01);
        // The fade 1 draws nothing.
        let none = paint_compass(&texture(), 4, 900, [0.0, 1.0], 1.0);
        assert!(none.pixels.iter().all(|p| p[3] == 0.0));
        // Half faded: 96 of 255.
        let half = paint_compass(&texture(), 4, 900, [0.0, 1.0], 0.5);
        assert!((half.get(56, 841).unwrap()[3] - 96.0 / 255.0).abs() < 0.01);
        // Outside the canvas nothing is drawn.
        assert_eq!(c.get(7, 841), None);
        assert_eq!(c.get(56, 889), None);
    }

    #[test]
    fn the_compass_turn_moves_the_picture() {
        // A quarter turn of the yaw: the picture's top left (blue) is at
        // the screen's top right, its top right (white) at the bottom right.
        let c = paint_compass(&texture(), 4, 900, [1.0, 0.0], 0.0);
        assert_eq!((c.x, c.y, c.width, c.height), (11, 796, 96, 96));
        let at = |x, y| c.get(x, y).unwrap();
        assert_eq!(&at(105, 797)[..3], &[0.0, 0.0, 1.0]);
        assert_eq!(&at(105, 890)[..3], &[1.0, 1.0, 1.0]);
        assert_eq!(&at(12, 797)[..3], &[1.0, 0.0, 0.0]);
        // At a diagonal heading the turned square reaches √2 times as far:
        // its corner is painted, not clipped.
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let c = paint_compass(&texture(), 4, 900, [d, d], 0.0);
        assert_eq!(c.width, 137);
        let top = c.y;
        let corner = c.get(c.x + 68, top + 1).unwrap();
        assert!(corner[3] > 0.0, "{corner:?}");
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
