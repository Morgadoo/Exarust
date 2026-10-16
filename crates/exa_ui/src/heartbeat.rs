//! The low-health heartbeat's border: a dark red frame round the game's
//! view, pulsing every 800 ms for 7 s, as the original draws it. The game
//! starts it ([`Heartbeat`]) and plays its sound; this module draws the
//! border. Rules: `docs/UI.md` (Heartbeat).

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::LoadingStatus;
use crate::blend::linear_opacity;
use crate::hud::DrawHud;

/// When the heartbeat started (real time, seconds), set by the game.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct Heartbeat {
    pub started: Option<f64>,
}

/// How long the heartbeat lasts (ms).
pub const HEARTBEAT_MS: u64 = 7000;
/// One pulse (ms): the border is strongest half-way through.
const PULSE_MS: u64 = 800;
/// The border's colour and its strongest opacity (of 255).
pub const BORDER: [u8; 3] = [80, 0, 0];
const BORDER_ALPHA: f32 = 96.0;
/// The border reaches this share of the window's height in from every edge.
const INSET: f64 = 0.2;
/// The border's image is drawn at this fraction of the window's size and
/// stretched: it is a linear ramp, which linear filtering keeps.
const DOWNSCALE: u32 = 8;

/// The border's opacity (of 255) `elapsed` ms after the heartbeat started:
/// 0 at the start of each 800 ms pulse, 96 half-way; `None` after 7 s.
pub fn pulse(elapsed: u64) -> Option<u8> {
    if elapsed > HEARTBEAT_MS {
        return None;
    }
    let half = (PULSE_MS / 2) as i64;
    let from_peak = (half - (elapsed % PULSE_MS) as i64).abs() as f32;
    let p = 1.0 - from_peak / half as f32;
    Some((p * p * BORDER_ALPHA).round_ties_even() as u8)
}

/// How far in the border reaches (pixels): a fifth of the window's height,
/// on the sides too.
pub fn inset(window_height: u32) -> u32 {
    (window_height as f64 * INSET).round_ties_even() as u32
}

/// The border's share of its opacity at a window point: 1 at the edge, 0 at
/// the inset and within, falling with the distance to the nearest edge.
pub fn coverage(point: Vec2, window: Vec2, inset: f32) -> f32 {
    if inset <= 0.0 {
        return 0.0;
    }
    let d = point
        .x
        .min(window.x - point.x)
        .min(point.y)
        .min(window.y - point.y);
    (1.0 - d / inset).clamp(0.0, 1.0)
}

/// The border's image for a window and an opacity: one texel per
/// [`DOWNSCALE`] pixels, sampled at the texels' centres.
pub fn border_image(width: u32, height: u32, alpha: u8) -> (UVec2, Vec<u8>) {
    let size = UVec2::new(width.div_ceil(DOWNSCALE), height.div_ceil(DOWNSCALE)).max(UVec2::ONE);
    let window = Vec2::new(width as f32, height as f32);
    let reach = inset(height) as f32;
    let colour = BORDER.map(|c| c as f32 / 255.0);
    let mut data = Vec::with_capacity((size.x * size.y * 4) as usize);
    for j in 0..size.y {
        for i in 0..size.x {
            let point = (Vec2::new(i as f32, j as f32) + 0.5) * window / size.as_vec2();
            let a = alpha as f32 / 255.0 * coverage(point, window, reach);
            let opacity = linear_opacity(colour, a);
            data.extend_from_slice(&[
                BORDER[0],
                BORDER[1],
                BORDER[2],
                (opacity * 255.0).round() as u8,
            ]);
        }
    }
    (size, data)
}

#[derive(Component)]
struct BorderNode;

/// The image drawn and what it was drawn for.
#[derive(Resource, Default)]
struct BorderState {
    painted: Option<(UVec2, u8)>,
    image: Option<Handle<Image>>,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Heartbeat>()
        .init_resource::<BorderState>()
        .add_systems(Startup, spawn)
        .add_systems(Update, draw.in_set(DrawHud));
}

/// Under the HUD's bars.
const Z: i32 = 890;

fn spawn(mut commands: Commands) {
    commands.spawn((
        Name::new("HeartbeatBorder"),
        BorderNode,
        Node {
            position_type: PositionType::Absolute,
            left: px(0.0),
            top: px(0.0),
            width: percent(100.0),
            height: percent(100.0),
            ..default()
        },
        ImageNode::default(),
        GlobalZIndex(Z),
        Visibility::Hidden,
    ));
}

fn draw(
    beat: Res<Heartbeat>,
    time: Res<Time<Real>>,
    loading: Option<Res<LoadingStatus>>,
    mut state: ResMut<BorderState>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&mut ImageNode, &mut Visibility), With<BorderNode>>,
) {
    let (Ok(window), Ok((mut sprite, mut visibility))) = (windows.single(), nodes.single_mut())
    else {
        return;
    };
    let elapsed = beat
        .started
        .map(|s| ((time.elapsed_secs_f64() - s).max(0.0) * 1000.0) as u64);
    let alpha = elapsed.and_then(pulse).filter(|_| loading.is_none());
    let Some(alpha) = alpha else {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let size = UVec2::new(window.physical_width(), window.physical_height());
    if state.painted != Some((size, alpha)) {
        let (texels, data) = border_image(size.x, size.y, alpha);
        let mut image = Image::new(
            Extent3d {
                width: texels.x,
                height: texels.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::linear();
        if let Some(handle) = state.image.clone()
            && let Some(mut slot) = images.get_mut(&handle)
        {
            *slot = image;
        } else {
            state.image = Some(images.add(image));
        }
        state.painted = Some((size, alpha));
    }
    if let Some(handle) = &state.image
        && sprite.image != *handle
    {
        sprite.image = handle.clone();
    }
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_border_pulses_every_800_ms_for_7_s() {
        assert_eq!(pulse(0), Some(0));
        assert_eq!(pulse(400), Some(96));
        assert_eq!(pulse(800), Some(0));
        assert_eq!(pulse(1200), Some(96));
        // A quarter pulse from the peak: (1 − 0.5)² × 96.
        assert_eq!(pulse(200), Some(24));
        assert_eq!(pulse(600), Some(24));
        assert_eq!(pulse(7000), Some(24));
        assert_eq!(pulse(7001), None);
    }

    #[test]
    fn the_border_reaches_a_fifth_of_the_height_in() {
        assert_eq!(inset(900), 180);
        assert_eq!(inset(1014), 203);
        let window = Vec2::new(1600.0, 900.0);
        assert_eq!(coverage(Vec2::new(0.0, 450.0), window, 180.0), 1.0);
        assert_eq!(coverage(Vec2::new(800.0, 90.0), window, 180.0), 0.5);
        // On the sides too, measured by the height's fifth.
        assert_eq!(coverage(Vec2::new(1555.0, 450.0), window, 180.0), 0.75);
        assert_eq!(coverage(Vec2::new(800.0, 450.0), window, 180.0), 0.0);
        // In a corner the nearer edge counts.
        assert_eq!(coverage(Vec2::new(45.0, 90.0), window, 180.0), 0.75);
    }

    #[test]
    fn the_border_image_is_dark_red_at_the_edges_only() {
        let (size, data) = border_image(1600, 900, 96);
        assert_eq!(size, UVec2::new(200, 113));
        let texel = |i: u32, j: u32| {
            let k = ((j * size.x + i) * 4) as usize;
            [data[k], data[k + 1], data[k + 2], data[k + 3]]
        };
        let edge = texel(0, 56);
        assert_eq!([edge[0], edge[1], edge[2]], BORDER);
        assert!(edge[3] > 0);
        assert_eq!(texel(100, 56)[3], 0);
        assert!(texel(0, 56)[3] > texel(10, 56)[3]);
        let (_, none) = border_image(1600, 900, 0);
        assert!(none.as_chunks::<4>().0.iter().all(|p| p[3] == 0));
    }
}
