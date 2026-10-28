//! The off-screen item icon renderer: Bevy draws the item's model into a
//! texture on a private render layer, the GPU readback hands the pixels
//! back, and the icon atlas takes the icon. At most [`ICONS_PER_FRAME`]
//! pending icons are started each frame, so opening a pack does not stall
//! the frame. The plan, camera and scene are [`exa_items::item_icon`] and
//! [`exa_items::item_renderer`]; a headless App paints the same scene on the
//! CPU ([`IconRenderMode::gpu`] false), which the tests use.
//!
//! The camera renders at twice the icon's side and the plan's screen pass
//! reduces it to the icon size (the alpha global's default); without the
//! screen pass the Kaiser downsample and the halo run instead.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use bevy::render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use exa_assets::model;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind};
use exa_items::catalogue::ItemCatalogue;
use exa_items::item_gui::framing;
use exa_items::item_icon::{self, IconCamera, IconPixels, IconScene, sun_direction};
use exa_items::item_renderer::ALPHA_GLOBAL;

use crate::item_panel_draw::{PanelIcons, PendingIcon};

/// How many icons one frame starts.
pub const ICONS_PER_FRAME: usize = 2;
/// The render layer the icon cameras and their scenes use, so the main view
/// never sees them.
pub const ICON_LAYER: usize = 30;
/// The frames a GPU icon may take before it is dropped (a lost readback
/// must not leak its scene).
pub const ICON_JOB_TIMEOUT: u32 = 60;
/// The frames before the readback is requested: the target image needs its
/// GPU image prepared, and the camera its first draws.
pub const READBACK_DELAY: u32 = 6;
/// The icon scene sun's colour and illuminance. The original's icon scene
/// has a sun of colour 0.75; `SUN_LUX` in the shading is 160, so a white
/// 120-lux light gives the same 0.75 irradiance.
pub const ICON_SUN: f32 = 120.0;

/// How an icon is painted: the off-screen GPU camera, or the CPU rasteriser
/// (a headless App, and the tests).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IconRenderMode {
    pub gpu: bool,
}

/// The icon passes, so the panel's paint can run after them.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconRenderSystems;

/// One icon being rendered by an off-screen camera.
#[derive(Debug)]
pub struct IconGpuJob {
    pub id: u32,
    pub key: u32,
    pub image: Handle<Image>,
    pub camera: Entity,
    pub light: Entity,
    pub root: Entity,
    pub readback: Option<Entity>,
    pub frames: u32,
    pub icon: u32,
    pub buffer: u32,
    pub screen_pass: bool,
    pub downsample: bool,
    pub halo: bool,
    pub halo_highlight: bool,
    pub content_width: f32,
    pub content_height: f32,
    /// The scene and camera, so the CPU can paint a stand-in when the
    /// readback has no covered pixel (an unsupported adapter or a lost
    /// draw).
    pub scene: IconScene,
    pub paint_camera: IconCamera,
    pub sun: Vec3,
}

/// The icons currently on the GPU.
#[derive(Resource, Debug, Default)]
pub struct IconGpuJobs {
    pub jobs: Vec<IconGpuJob>,
}

/// The icon renderer plugin. The game adds it after the GUI screens; a
/// headless App gets the CPU painter.
pub struct ItemIconPlugin;

impl Plugin for ItemIconPlugin {
    fn build(&self, app: &mut App) {
        let gpu = app.get_sub_app(bevy::render::RenderApp).is_some();
        info!(
            "item icons: {} renderer",
            if gpu { "off-screen" } else { "CPU" }
        );
        app.insert_resource(IconRenderMode { gpu })
            .init_resource::<PanelIcons>()
            .init_resource::<IconGpuJobs>()
            .add_systems(
                Update,
                (
                    prepare_icon_scenes,
                    paint_icon_jobs,
                    start_icon_readbacks,
                    expire_icon_jobs,
                )
                    .chain()
                    .in_set(IconRenderSystems),
            )
            .add_observer(icon_readback_complete);
    }
}

/// Loads the model of each pending icon, at most [`ICONS_PER_FRAME`] a
/// frame, and bakes its scene. A model that cannot be loaded is retried
/// twice, then the pending icon is dropped (its slot stays reserved).
#[allow(clippy::too_many_arguments)]
fn prepare_icon_scenes(
    mut icons: ResMut<PanelIcons>,
    game: Option<Res<Game>>,
    catalogue: Option<Res<ItemCatalogue>>,
    cache: Option<ResMut<ModelCache>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<exa_render::material::ExaMaterial>>>,
    images: Option<ResMut<Assets<Image>>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let (
        Some(game),
        Some(catalogue),
        Some(mut cache),
        Some(mut meshes),
        Some(mut materials),
        Some(mut images),
    ) = (game, catalogue, cache, meshes, materials, images)
    else {
        return;
    };
    if catalogue.is_empty() {
        // The catalogue loads once the game data is in; a pending icon
        // waits rather than being taken for an unknown record.
        return;
    }
    let mut budget = ICONS_PER_FRAME;
    let mut index = 0;
    while index < icons.pending.len() && budget > 0 {
        if icons.pending[index].scene.is_some() {
            index += 1;
            continue;
        }
        let id = icons.pending[index].id;
        let Some(model_info) = catalogue.model(id).cloned() else {
            icons.pending.remove(index);
            continue;
        };
        let mut loader = ModelLoader {
            game: &game.0,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: model::bc_supported(support.as_deref()),
        };
        let model = match model_info.model.strip_prefix('@') {
            Some(factory) => loader.factory_weapon(factory, model_info.weapon.as_ref()),
            None => [PackKind::Objects, PackKind::Objlib]
                .into_iter()
                .find(|&pack| {
                    loader
                        .game
                        .pack(pack)
                        .is_some_and(|archive| archive.get(&model_info.model).is_some())
                })
                .and_then(|pack| loader.model(pack, &model_info.model)),
        };
        let Some(model) = model else {
            let pending = &mut icons.pending[index];
            pending.attempts += 1;
            if pending.attempts >= 3 {
                icons.pending.remove(index);
                continue;
            }
            index += 1;
            continue;
        };
        let variant = icons.pending[index].variant;
        let turned = framing(variant, false).turned;
        let hide_star = model_info.flags & 0x5f == 9;
        let scene = IconScene::from_model(&model, &meshes, &materials, hide_star, turned);
        let Some(scene) = scene else {
            let pending = &mut icons.pending[index];
            pending.attempts += 1;
            if pending.attempts >= 3 {
                icons.pending.remove(index);
                continue;
            }
            index += 1;
            continue;
        };
        icons.pending[index].scene = Some(scene);
        budget -= 1;
        index += 1;
    }
}

/// Starts the prepared icons: the GPU camera, or the CPU paint. The plan is
/// the reviewed render plan; the camera and extents come from the same
/// scene bounds, so the icon is framed exactly as the plan says.
fn paint_icon_jobs(
    mut commands: Commands,
    mut icons: ResMut<PanelIcons>,
    mode: Res<IconRenderMode>,
    mut jobs: ResMut<IconGpuJobs>,
    mut images: Option<ResMut<Assets<Image>>>,
    cache: Option<Res<ModelCache>>,
) {
    // The original flushes its procedural texture jobs before the render;
    // Exarust waits for the streamed uploads so the icon does not keep the
    // loader's grey placeholder.
    if mode.gpu
        && cache
            .as_ref()
            .is_some_and(|cache| cache.textures.entries().iter().any(|record| record.pending))
    {
        return;
    }
    let mut budget = ICONS_PER_FRAME;
    let mut index = 0;
    while index < icons.pending.len() && budget > 0 {
        if icons.pending[index].scene.is_none() {
            index += 1;
            continue;
        }
        let pending = icons.pending.remove(index).expect("index is in range");
        budget -= 1;
        let Some(scene) = pending.scene.as_ref() else {
            continue;
        };
        let icon = icons.atlas.cell.max(1) as u32;
        let (camera, plan) = item_icon::scene_plan(
            pending.variant,
            false,
            pending.model_flags,
            scene.bounds,
            icon as i32,
            ALPHA_GLOBAL,
        );
        if mode.gpu
            && let Some(images) = images.as_deref_mut()
            && let Some(job) =
                spawn_gpu_icon(&mut commands, images, &pending, scene, &camera, &plan, icon)
        {
            jobs.jobs.push(job);
            continue;
        }
        let icon_pixels = scene.paint(&camera, sun_direction(plan.framing), icon);
        icons.apply_rendered(
            pending.id,
            pending.key,
            icon_pixels,
            plan.screen_extents.0,
            plan.screen_extents.1,
        );
    }
}

/// Spawns the off-screen camera, its sun and the model's parts on the icon
/// layer, and the image it renders into.
fn spawn_gpu_icon(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    pending: &PendingIcon,
    scene: &IconScene,
    view: &IconCamera,
    plan: &exa_items::item_renderer::RenderPlan,
    icon: u32,
) -> Option<IconGpuJob> {
    let buffer = icon.saturating_mul(2).max(1);
    let mut image = Image::new_uninit(
        Extent3d {
            width: buffer,
            height: buffer,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let image = images.add(image);
    let layer = RenderLayers::layer(ICON_LAYER);
    let camera = commands
        .spawn((
            Name::new("_ItemIconCamera"),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Projection::Perspective(PerspectiveProjection {
                fov: view.fov_degrees.to_radians(),
                near: view.near,
                far: view.far,
                aspect_ratio: 1.0,
                ..default()
            }),
            Transform::from_translation(view.position).looking_at(view.target, Vec3::Y),
            Tonemapping::None,
            Msaa::Off,
            layer.clone(),
        ))
        .id();
    let sun = sun_direction(plan.framing).normalize_or_zero();
    let up = if sun.abs().dot(Vec3::Y) > 0.999 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let light = commands
        .spawn((
            Name::new("_ItemIconSun"),
            DirectionalLight {
                color: Color::WHITE,
                illuminance: ICON_SUN,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default().looking_to(-sun, up),
            layer.clone(),
        ))
        .id();
    let root = commands
        .spawn((
            Name::new("_ItemIcon"),
            Transform {
                translation: -scene.centre,
                rotation: scene.turn,
                ..default()
            },
            Visibility::default(),
            layer.clone(),
        ))
        .id();
    for part in &scene.parts {
        commands.spawn((
            Mesh3d(part.mesh.clone()),
            MeshMaterial3d(part.material.clone()),
            Transform::from_matrix(part.transform),
            layer.clone(),
            ChildOf(root),
        ));
    }
    Some(IconGpuJob {
        id: pending.id,
        key: pending.key,
        image,
        camera,
        light,
        root,
        readback: None,
        frames: 0,
        icon,
        buffer,
        screen_pass: plan.screen_pass,
        downsample: plan.downsample,
        halo: plan.halo,
        halo_highlight: plan.halo_highlight,
        content_width: plan.screen_extents.0,
        content_height: plan.screen_extents.1,
        scene: scene.clone(),
        paint_camera: *view,
        sun,
    })
}

/// Requests the readback one frame after the camera has drawn.
fn start_icon_readbacks(mut commands: Commands, mut jobs: ResMut<IconGpuJobs>) {
    for job in &mut jobs.jobs {
        job.frames = job.frames.saturating_add(1);
        if job.frames == READBACK_DELAY && job.readback.is_none() {
            let entity = commands
                .spawn((
                    Name::new("_ItemIconReadback"),
                    ReadbackOnce::texture(job.image.clone()),
                ))
                .id();
            job.readback = Some(entity);
        }
    }
}

/// Drops a GPU icon that never answered, so its scene and image do not
/// leak; the CPU's paint of the same scene fills the slot, so a lost
/// readback still leaves an icon.
fn expire_icon_jobs(
    mut commands: Commands,
    mut jobs: ResMut<IconGpuJobs>,
    mut icons: ResMut<PanelIcons>,
) {
    let (keep, expired): (Vec<_>, Vec<_>) = std::mem::take(&mut jobs.jobs)
        .into_iter()
        .partition(|job| job.frames < ICON_JOB_TIMEOUT);
    jobs.jobs = keep;
    for job in expired {
        let mut icon = job.scene.paint(&job.paint_camera, job.sun, job.icon);
        if job.halo {
            exa_items::item_gui::halo(
                &mut icon.pixels,
                icon.width as usize,
                icon.height as usize,
                job.halo_highlight,
            );
        }
        icons.apply_rendered(job.id, job.key, icon, job.content_width, job.content_height);
        commands.entity(job.camera).despawn();
        commands.entity(job.light).despawn();
        commands.entity(job.root).despawn();
        if let Some(readback) = job.readback {
            commands.entity(readback).despawn();
        }
    }
}

/// The GPU readback: the icon image's pixels become an atlas cell.
fn icon_readback_complete(
    trigger: On<ReadbackComplete>,
    mut icons: ResMut<PanelIcons>,
    mut jobs: ResMut<IconGpuJobs>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let readback = trigger.event().entity;
    let Some(at) = jobs
        .jobs
        .iter()
        .position(|job| job.readback == Some(readback))
    else {
        return;
    };
    let job = jobs.jobs.swap_remove(at);
    let nonzero = trigger.event().data.iter().filter(|&&b| b != 0).count();
    eprintln!(
        "readback: {} bytes, {nonzero} nonzero, first {:?}",
        trigger.event().data.len(),
        &trigger.event().data[..8.min(trigger.event().data.len())]
    );
    if let Some(icon) = icon_from_readback(
        &trigger.event().data,
        job.buffer,
        job.icon,
        job.screen_pass,
        job.downsample,
    ) {
        let mut icon = icon;
        if job.halo {
            exa_items::item_gui::halo(
                &mut icon.pixels,
                icon.width as usize,
                icon.height as usize,
                job.halo_highlight,
            );
        }
        icons.apply_rendered(job.id, job.key, icon, job.content_width, job.content_height);
    }
    for entity in [job.camera, job.light, job.root] {
        commands.entity(entity).despawn();
    }
    if let Some(readback) = job.readback {
        commands.entity(readback).despawn();
    }
    images.remove(&job.image);
}

/// The icon pixels of a readback. The GPU's rows run top-down and the atlas
/// stores bottom-up. The screen pass samples the 2× buffer at each icon
/// pixel's centre, a 2 × 2 bilinear average in linear light; the
/// non-screen path runs the Kaiser downsample at the icon's image format.
fn icon_from_readback(
    data: &[u8],
    buffer: u32,
    icon: u32,
    screen_pass: bool,
    downsample: bool,
) -> Option<IconPixels> {
    let width = buffer as usize;
    let height = buffer as usize;
    let stride = (width * 4).next_multiple_of(256);
    if width == 0 || height == 0 || icon == 0 || data.len() < stride * height {
        return None;
    }
    let mut source = vec![0u8; width * height * 4];
    for y in 0..height {
        let row = (height - 1 - y) * stride;
        source[y * width * 4..(y + 1) * width * 4].copy_from_slice(&data[row..row + width * 4]);
    }
    let icon = icon as usize;
    let mut pixels = if screen_pass {
        resolve_icon(&source, width, height, icon)
    } else if downsample {
        exa_render::mipmap::resample_rgba8_to_size(
            &source,
            width,
            height,
            icon,
            icon,
            exa_items::item_renderer::IMAGE_FORMAT,
            exa_items::item_renderer::IMAGE_FLAGS,
        )
    } else if width == icon && height == icon {
        source
    } else {
        resolve_icon(&source, width, height, icon)
    };
    if pixels.len() < icon * icon * 4 {
        return None;
    }
    pixels.truncate(icon * icon * 4);
    Some(IconPixels {
        width: icon as u32,
        height: icon as u32,
        pixels,
    })
}

/// A 2 × 2 box resolve in linear light, the screen pass's bilinear sample
/// at each destination pixel's centre [guess: the original's screen shader
/// sampler is not traced; a 2× reduction's midpoint sample is a 2 × 2
/// average].
fn resolve_icon(source: &[u8], width: usize, height: usize, icon: usize) -> Vec<u8> {
    let mut out = vec![0u8; icon * icon * 4];
    let ratio_x = width as f32 / icon as f32;
    let ratio_y = height as f32 / icon as f32;
    for y in 0..icon {
        for x in 0..icon {
            let mut sum = [0.0f32; 3];
            let mut alpha = 0.0f32;
            let mut count = 0.0f32;
            let start_x = (x as f32 * ratio_x).floor() as usize;
            let start_y = (y as f32 * ratio_y).floor() as usize;
            let taps_x = (ratio_x.ceil() as usize).max(1);
            let taps_y = (ratio_y.ceil() as usize).max(1);
            for sy in start_y..(start_y + taps_y).min(height) {
                for sx in start_x..(start_x + taps_x).min(width) {
                    let at = (sy * width + sx) * 4;
                    sum[0] += srgb_to_linear(source[at]);
                    sum[1] += srgb_to_linear(source[at + 1]);
                    sum[2] += srgb_to_linear(source[at + 2]);
                    alpha += f32::from(source[at + 3]) / 255.0;
                    count += 1.0;
                }
            }
            if count == 0.0 {
                continue;
            }
            let at = (y * icon + x) * 4;
            for channel in 0..3 {
                out[at + channel] = linear_to_srgb(sum[channel] / count);
            }
            out[at + 3] = (alpha / count * 255.0).round() as u8;
        }
    }
    out
}

fn srgb_to_linear(byte: u8) -> f32 {
    let c = f32::from(byte) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let c = value.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_readback_resolves_to_the_icon_size_bottom_up() {
        // A 4 × 4 buffer of one colour resolves to 2 × 2; the readback is
        // top-down and the atlas wants bottom-up, so the first source row's
        // pixels end in the last atlas row.
        let buffer = 4u32;
        let stride = (buffer as usize * 4).next_multiple_of(256);
        let mut data = vec![0u8; stride * buffer as usize];
        for y in 0..buffer as usize {
            for x in 0..buffer as usize {
                let at = y * stride + x * 4;
                data[at] = 10;
                data[at + 1] = 20;
                data[at + 2] = 30;
                data[at + 3] = 255;
            }
        }
        let icon = icon_from_readback(&data, buffer, 2, true, false).unwrap();
        assert_eq!((icon.width, icon.height), (2, 2));
        assert_eq!(icon.alpha(0, 0), 255);
        assert_eq!(icon.alpha(1, 1), 255);
        assert_eq!(icon.pixels[0], 10);
        // A short buffer is refused.
        assert!(icon_from_readback(&data[..10], buffer, 2, true, false).is_none());
    }

    /// The installed object database's first ground-modelled weapon fills
    /// the pack's atlas through the pending pipeline (skipped without an
    /// installation).
    #[test]
    fn a_shipped_weapons_icon_reaches_the_panel_atlas() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let Some(bytes) = data
            .pack(exa_assets::PackKind::Resource)
            .and_then(|pack| pack.get("objdb.rdb"))
        else {
            return;
        };
        let Ok(db) = exa_formats::rdb::Database::parse(bytes) else {
            return;
        };
        let items = exa_formats::objdb::items(&db);
        let Some(item) = items
            .iter()
            .find(|item| item.model.starts_with('@') && item.weapon.is_some())
        else {
            return;
        };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<exa_render::material::ExaMaterial>()
            .init_asset::<Image>()
            .init_resource::<ModelCache>()
            .init_resource::<PanelIcons>()
            .init_resource::<IconGpuJobs>()
            .insert_resource(IconRenderMode { gpu: false })
            .insert_resource(ItemCatalogue::of_database(&db))
            .insert_resource(Game(std::sync::Arc::new(data)))
            .add_systems(
                Update,
                (
                    prepare_icon_scenes,
                    paint_icon_jobs,
                    start_icon_readbacks,
                    expire_icon_jobs,
                )
                    .chain(),
            );
        let catalogue = app.world().resource::<ItemCatalogue>().clone();
        let mut icons = app.world_mut().resource_mut::<PanelIcons>();
        let (handle, _) = icons.reserve(item.id, 0, Some(&catalogue));
        assert_eq!(handle, 1);
        for _ in 0..4 {
            app.update();
        }
        let icons = app.world().resource::<PanelIcons>();
        assert!(
            icons.cache.entries[0].cell >= 1,
            "{} did not render: {:?}",
            item.name,
            icons.cache.entries[0]
        );
        assert!(icons.atlas.image.pixels.iter().any(|&b| b != 0));
    }

    #[test]
    fn the_icon_plugin_caches_a_pending_icon() {
        let mut app = App::new();
        app.init_resource::<PanelIcons>()
            .init_resource::<IconRenderMode>()
            .init_resource::<IconGpuJobs>()
            .add_systems(
                Update,
                (
                    prepare_icon_scenes,
                    paint_icon_jobs,
                    start_icon_readbacks,
                    expire_icon_jobs,
                )
                    .chain()
                    .in_set(IconRenderSystems),
            );
        app.world_mut().resource_mut::<IconRenderMode>().gpu = false;
        let mut icons = app.world_mut().resource_mut::<PanelIcons>();
        let (handle, _) = icons.reserve(7, 0, None);
        assert_eq!(handle, 1);
        assert_eq!(icons.cache.entries[0].cell, -1);
        icons.pending[0].scene = Some(IconScene::synthetic_quad());
        app.update();
        let icons = app.world().resource::<PanelIcons>();
        assert!(
            icons.cache.entries[0].cell >= 1,
            "{:?}",
            icons.cache.entries[0]
        );
        assert!(icons.atlas.find(7) >= 1);
        assert!(icons.atlas.image.pixels.iter().any(|&b| b != 0));
        assert!(icons.pending.is_empty());
        // The panel draws the filled cell through its HltCol layer.
        let mut container = crate::item_container::Container::form(4);
        container.entries = vec![crate::item_container::Entry {
            cell: crate::item_container::Cell::new(10, 20),
            id: 7,
            icon: 1,
            flags: 0,
            drag: 0,
        }];
        container.icon_size = 160;
        container.green_mask = 0x400;
        container.yellow_mask = 0x200;
        let mut icons = app.world_mut().resource_mut::<PanelIcons>();
        let layer = icons.layer(&container).expect("a filled icon draws");
        assert_eq!(layer.batch.primitives.len(), 1);
    }

    #[test]
    fn an_expired_gpu_job_falls_back_to_the_cpu_paint() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Image>()
            .init_resource::<PanelIcons>()
            .init_resource::<IconGpuJobs>()
            .add_systems(Update, expire_icon_jobs);
        let entry = {
            let mut icons = app.world_mut().resource_mut::<PanelIcons>();
            let (handle, _) = icons.reserve(9, 0, None);
            icons.cache.entries[handle - 1].variant
        };
        let scene = IconScene::synthetic_quad();
        let (view, _) = item_icon::scene_plan(0, false, 0, scene.bounds, 160, true);
        let world = app.world_mut();
        let camera = world.spawn_empty().id();
        let light = world.spawn_empty().id();
        let root = world.spawn_empty().id();
        let image = world.resource_mut::<Assets<Image>>().add(Image::default());
        world.resource_mut::<IconGpuJobs>().jobs.push(IconGpuJob {
            id: 9,
            key: entry,
            image,
            camera,
            light,
            root,
            readback: None,
            frames: ICON_JOB_TIMEOUT,
            icon: 160,
            buffer: 320,
            screen_pass: true,
            downsample: false,
            halo: false,
            halo_highlight: false,
            content_width: 144.0,
            content_height: 144.0,
            scene,
            paint_camera: view,
            sun: sun_direction(framing(0, false)),
        });
        app.update();
        let icons = app.world().resource::<PanelIcons>();
        assert!(icons.cache.entries[0].cell >= 1);
        assert!(icons.atlas.image.pixels.iter().any(|&b| b != 0));
        assert!(app.world().resource::<IconGpuJobs>().jobs.is_empty());
    }
}
