//! Supersampling as the original does it at its 4× setting: the scene is
//! drawn at twice the window's resolution on each axis and resolved to
//! the window by an equal-weight average of each 2 × 2 block (the original
//! draws four jittered sub-frames and averages them), then post-processed
//! at window resolution. A [`Supersample`] scene camera renders into an
//! image; a display camera on its own render layer draws that image back
//! through the post chain. Rules: `docs/RENDERING.md` (Supersampling).

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, Hdr, RenderTarget, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::shader::ShaderRef;
use bevy::window::PrimaryWindow;
use bevy::window::WindowRef;

use crate::post::ExaPost;

pub struct SupersamplePlugin;

impl Plugin for SupersamplePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "resolve.wesl");
        app.add_plugins((
            MaterialPlugin::<ResolveMaterial>::default(),
            ExtractComponentPlugin::<DisplayCamera>::default(),
        ))
        .init_resource::<SplitTaps>()
        .init_resource::<SuperSampling>()
        .add_systems(
            PostUpdate,
            (
                sync_supersampling,
                start_supersampling,
                follow_window,
                sync_split_taps::<crate::material::ExaMaterial>,
            )
                .chain(),
        );
    }
}

/// The settings screen's super-sampling switch: whether the scene is drawn
/// at twice the window's resolution on each axis. Turning it off tears the
/// scene's target down and gives the camera the window back; turning it on
/// adds [`Supersample`] to the scene cameras, which builds the target.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuperSampling(pub bool);

impl Default for SuperSampling {
    fn default() -> Self {
        Self(true)
    }
}

/// Whether the scene is supersampled, so that each sub-frame takes its
/// share of the shadow and GI taps, as the original spreads them over its
/// sub-frames. Surface materials follow it through [`SplitTapsFlag`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitTaps(pub bool);

/// A material whose shader splits its taps when its flag bit is set.
pub trait SplitTapsFlag: Asset {
    const BIT: u32;
    fn flags(&self) -> u32;
    fn flags_mut(&mut self) -> &mut u32;
}

impl SplitTapsFlag for crate::material::ExaMaterial {
    const BIT: u32 = crate::material::flags::SPLIT_TAPS;
    fn flags(&self) -> u32 {
        self.params.flags
    }
    fn flags_mut(&mut self) -> &mut u32 {
        &mut self.params.flags
    }
}

/// Keeps every material's split-taps bit equal to [`SplitTaps`].
pub fn sync_split_taps<M: SplitTapsFlag>(
    split: Res<SplitTaps>,
    mut materials: ResMut<Assets<M>>,
    mut events: MessageReader<AssetEvent<M>>,
) {
    let added = events.read().filter_map(|e| match e {
        AssetEvent::Added { id } => Some(*id),
        _ => None,
    });
    let ids: Vec<AssetId<M>> = if split.is_changed() {
        materials.ids().collect()
    } else {
        added.collect()
    };
    for id in ids {
        let Some(m) = materials.get(id) else { continue };
        if (m.flags() & M::BIT != 0) == split.0 {
            continue;
        }
        if let Some(mut m) = materials.get_mut(id) {
            *m.flags_mut() ^= M::BIT;
        }
    }
}

/// A scene camera drawn at twice the window's resolution and resolved to it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Supersample;

/// The camera showing a supersampled scene: it draws only the resolve quad
/// and carries the post-processing. World systems (GI, flames, fog) ignore
/// it.
#[derive(Component, ExtractComponent, Clone, Copy, Debug)]
#[extract_app(bevy::render::RenderApp)]
pub struct DisplayCamera;

/// Query filter for the camera that sees the world: a 3D camera that is not
/// a supersampling display.
pub type SceneCamera = (With<Camera3d>, Without<DisplayCamera>);

/// The render layer of the resolve quad. Layers above it are free for the
/// lights' shadow casters.
pub const RESOLVE_LAYER: usize = 1;
/// Scene resolution per window pixel, on each axis.
const SCALE: u32 = 2;

/// The supersampled scene's image, and the quad showing it.
#[derive(Component)]
struct SupersampleTarget {
    image: Handle<Image>,
    material: Handle<ResolveMaterial>,
    size: UVec2,
    /// The camera drawing the resolve quad over the window.
    display: Entity,
    /// The mesh entity of the resolve quad.
    resolve: Entity,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ResolveMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub scene: Handle<Image>,
}

impl Material for ResolveMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("resolve.wesl")).with_source("embedded"),
        )
    }
}

fn scene_image(size: UVec2) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST;
    // Bilinear at each window pixel's centre averages its 2 × 2 block.
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    image
}

fn window_size(window: &Window) -> UVec2 {
    UVec2::new(window.physical_width(), window.physical_height())
}

#[allow(clippy::too_many_arguments)]
fn start_supersampling(
    mut commands: Commands,
    mut split: ResMut<SplitTaps>,
    cameras: Query<(Entity, &Camera), Added<Supersample>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ResolveMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(window) = windows.single() else { return };
    for (entity, camera) in &cameras {
        split.0 = true;
        let size = window_size(window) * SCALE;
        let image = images.add(scene_image(size));
        commands
            .entity(entity)
            .insert((
                RenderTarget::Image(image.clone().into()),
                Hdr,
                Tonemapping::None,
                // The original multisamples nothing (`HWMultisample` 0):
                // the sub-frames are its only antialiasing.
                Msaa::Off,
            ))
            .remove::<ExaPost>();
        let material = materials.add(ResolveMaterial {
            scene: image.clone(),
        });
        let layer = RenderLayers::layer(RESOLVE_LAYER);
        let resolve = commands
            .spawn((
                Name::new("_SupersampleResolve"),
                Mesh3d(meshes.add(Rectangle::new(2.0, 2.0))),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                layer.clone(),
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
            ))
            .id();
        let display = commands
            .spawn((
                Name::new("_SupersampleDisplay"),
                Camera3d::default(),
                Camera {
                    order: camera.order + 1,
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    ..default()
                },
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: 2.0,
                        height: 2.0,
                    },
                    ..OrthographicProjection::default_3d()
                }),
                Transform::from_xyz(0.0, 0.0, 1.0),
                layer,
                DisplayCamera,
            ))
            .id();
        commands.entity(entity).insert(SupersampleTarget {
            image,
            material,
            size,
            display,
            resolve,
        });
    }
}

/// Follows [`SuperSampling`]: turning it on gives every scene camera a
/// [`Supersample`] (the start system builds its target); turning it off
/// removes it, despawns the resolve quad and its display camera and gives the
/// camera the window back with its post-processing.
fn sync_supersampling(
    mut commands: Commands,
    sampling: Res<SuperSampling>,
    mut split: ResMut<SplitTaps>,
    cameras: Query<(Entity, Option<&SupersampleTarget>), SceneCamera>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ResolveMaterial>>,
) {
    if !sampling.is_changed() {
        return;
    }
    if sampling.0 {
        split.0 = true;
        for (entity, target) in &cameras {
            if target.is_none() {
                commands.entity(entity).insert(Supersample);
            }
        }
        return;
    }
    split.0 = false;
    for (entity, target) in &cameras {
        commands.entity(entity).remove::<Supersample>();
        let Some(target) = target else { continue };
        commands.entity(target.display).despawn();
        commands.entity(target.resolve).despawn();
        images.remove(&target.image);
        materials.remove(&target.material);
        commands
            .entity(entity)
            .insert((
                RenderTarget::Window(WindowRef::Primary),
                Hdr,
                Tonemapping::None,
                crate::post::ExaPost::default(),
            ))
            .remove::<SupersampleTarget>();
    }
}

/// Rebuilds the scene image whenever the window's size differs from it
/// (checked every frame: the window may settle after the camera starts).
fn follow_window(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<(&mut RenderTarget, &mut SupersampleTarget)>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ResolveMaterial>>,
) {
    let Ok(window) = windows.single() else { return };
    let size = window_size(window) * SCALE;
    for (mut target, mut ss) in &mut cameras {
        if ss.size == size {
            continue;
        }
        let image = images.add(scene_image(size));
        *target = RenderTarget::Image(image.clone().into());
        if let Some(mut m) = materials.get_mut(&ss.material) {
            m.scene = image.clone();
        }
        images.remove(&ss.image);
        ss.image = image;
        ss.size = size;
    }
}
