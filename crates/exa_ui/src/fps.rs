//! The frame-rate readout in the top-right corner, as the original shows it
//! ("57.25 FPS"), drawn in the game's base font. Rules: `docs/UI.md` (Frame
//! rate).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::Game;
use exa_formats::rfi::Rfi;

use crate::font::{BASE_FONT, GameFont};

/// Whether the frame rate is shown; on by default.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ShowFps(pub bool);

impl Default for ShowFps {
    fn default() -> Self {
        Self(true)
    }
}

/// Window pixels per atlas pixel: the 64-pixel cells drawn at the font's
/// 24-point size. The original draws in physical pixels, whatever the
/// display's scale factor.
const SCALE: f32 = 0.47;
/// Seconds between readings; each is the mean rate over that time.
const PERIOD: f32 = 0.5;
const COLOUR: Color = Color::srgb(0.66, 0.66, 0.66);
/// Distance from the window's top and right edges, in window pixels.
const MARGIN_TOP: f32 = 2.0;
const MARGIN_RIGHT: f32 = 10.0;

#[derive(Resource)]
struct FpsFont(GameFont);

#[derive(Component, Default)]
struct FpsReadout {
    frames: u32,
    seconds: f32,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<ShowFps>()
        .add_systems(Startup, spawn)
        .add_systems(Update, update);
}

fn spawn(mut commands: Commands, game: Option<Res<Game>>, show: Res<ShowFps>) {
    if !show.0 {
        return;
    }
    let Some(game) = game else { return };
    let font = game
        .0
        .texture(BASE_FONT)
        .and_then(|b| Rfi::parse(b).ok())
        .and_then(|rfi| GameFont::from_rfi(&rfi));
    let Some(font) = font else {
        warn!("{BASE_FONT} missing: no frame-rate readout");
        return;
    };
    commands.insert_resource(FpsFont(font));
    commands.spawn((
        Name::new("FpsReadout"),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode {
            color: COLOUR,
            ..default()
        },
        Visibility::Hidden,
        FpsReadout::default(),
    ));
}

fn update(
    time: Res<Time<Real>>,
    font: Option<Res<FpsFont>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut readouts: Query<(&mut FpsReadout, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let Some(font) = font else { return };
    // Logical pixels per window pixel.
    let unit = windows.single().map_or(1.0, |w| 1.0 / w.scale_factor());
    for (mut readout, mut image, mut node, mut visibility) in &mut readouts {
        readout.frames += 1;
        readout.seconds += time.delta_secs();
        if readout.seconds < PERIOD {
            continue;
        }
        let fps = readout.frames as f32 / readout.seconds;
        *readout = FpsReadout::default();
        // Two decimals with the trailing zeros kept, as the original's
        // readout asks for ("60.00 FPS").
        let text = exa_formats::text::format_float(fps, 2, true);
        debug!("{text} FPS");
        let line = font.0.render(&format!("{text} FPS"));
        node.width = px(line.width as f32 * SCALE * unit);
        node.height = px(line.height as f32 * SCALE * unit);
        node.top = px(MARGIN_TOP * unit);
        node.right = px(MARGIN_RIGHT * unit);
        image.image = images.add(text_image(line.width, line.height, &line.alpha));
        *visibility = Visibility::Inherited;
    }
}

/// White text with the coverage as alpha; the node's colour tints it.
pub(crate) fn text_image(width: usize, height: usize, alpha: &[u8]) -> Image {
    let data = alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect();
    Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
