//! The off-screen item icon renderer: Bevy draws the item's model into a
//! texture on a private render layer, the GPU readback hands the pixels
//! back, and the icon atlas takes the icon. At most [`ICONS_PER_FRAME`]
//! pending icons are started each frame, so opening a pack does not stall
//! the frame. The plan, camera and scene are [`exa_items::item_icon`] and
//! [`exa_items::item_renderer`]; a headless App paints the same scene on the
//! CPU ([`IconRenderMode::gpu`] false), which the tests use; a readback with
//! no covered pixel, or one that times out, also takes that CPU paint.
//!
//! The camera renders at twice the icon's side and the plan's screen pass
//! reduces it to the icon size (the alpha global's default); without the
//! screen pass the Kaiser downsample and the halo run instead.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::CompressedImageFormatSupport;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use exa_assets::model::{self, Paint, PaintedSet};
use exa_assets::{Game, ModelCache, ModelLoader, PackKind};
use exa_formats::objdb::Appearance;
use exa_items::catalogue::ItemCatalogue;
use exa_items::item_gui::framing;
use exa_items::item_icon::{self, IconCamera, IconPixels, IconScene, sun_direction};
use exa_items::item_renderer::ALPHA_GLOBAL;
use exa_render::supersample::Offscreen;

use crate::item_panel_draw::{PanelIcons, PendingIcon};

/// How many icons one frame starts.
pub const ICONS_PER_FRAME: usize = 2;
/// The render layer the icon cameras and their scenes use: the off-screen
/// layer, which no scene camera and no light's shadow layer takes, so the
/// main view never sees them.
pub const ICON_LAYER: usize = exa_render::supersample::OFFSCREEN_LAYER;
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

/// The frames a prepared icon waits for its materials' textures before
/// it renders anyway (a shipped image that cannot be read keeps the 1 × 1
/// placeholder for good).
pub const TEXTURE_WAIT: u32 = 120;

/// Whether every texture the scene's materials bind is in the image store
/// and is not the streamed load's 1 × 1 placeholder. A GPU icon rendered before then draws the placeholder (a
/// flat grey-white model), and the readback is kept as the icon.
pub fn scene_textures_ready(
    scene: &IconScene,
    materials: &Assets<exa_render::material::ExaMaterial>,
    images: &Assets<Image>,
) -> bool {
    scene.parts.iter().all(|part| {
        let Some(material) = materials.get(&part.material) else {
            return false;
        };
        [
            &material.diffuse_map,
            &material.normal_map,
            &material.specular_map,
            &material.emissive_map,
            &material.opacity_map,
        ]
        .into_iter()
        .flatten()
        .all(|handle| {
            // A render-world image keeps its size after its pixels go to
            // the GPU, so the size alone tells the placeholder apart.
            images
                .get(handle)
                .is_some_and(|image| image.width().max(image.height()) > 1)
        })
    })
}

/// How long one frame may spend decoding the textures an icon's paint job
/// reads: a texture set's dozen textures take too long for one frame.
pub const ICON_PAINT_BUDGET: std::time::Duration = std::time::Duration::from_millis(4);

/// The apparel paint jobs icons wait on, by texture stem and appearance:
/// the original's streamed icon paints the item's run-time materials from
/// its body section's appearance (512 texels a side, 1024 at the highest
/// texture quality) before it renders; Exarust paints on the async compute
/// pool at the shade map's size and the icon waits for the material.
#[derive(Resource, Default)]
pub struct IconPainting {
    pub tasks: std::collections::HashMap<(String, Appearance), Task<PaintedSet>>,
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
            .init_resource::<IconPainting>()
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
/// twice, then the pending icon is dropped (its slot stays reserved). An
/// apparel model's run-time materials are painted from the item's
/// appearance first; the icon waits while its paint job runs.
#[allow(clippy::too_many_arguments)]
fn prepare_icon_scenes(
    mut icons: ResMut<PanelIcons>,
    mut painting: Option<ResMut<IconPainting>>,
    game: Option<Res<Game>>,
    catalogue: Option<Res<ItemCatalogue>>,
    cache: Option<ResMut<ModelCache>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<exa_render::material::ExaMaterial>>>,
    images: Option<ResMut<Assets<Image>>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    if icons.pending.iter().all(|pending| pending.scene.is_some()) {
        return;
    }
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
    let deadline = std::time::Instant::now() + ICON_PAINT_BUDGET;
    if let Some(painting) = painting.as_deref_mut()
        && !painting.tasks.is_empty()
    {
        let mut loader = ModelLoader {
            game: &game.0,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: model::bc_supported(support.as_deref()),
        };
        painting
            .tasks
            .retain(|_, task| match block_on(poll_once(task)) {
                Some(painted) => {
                    loader.painted_material(painted);
                    false
                }
                None => true,
            });
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
        let painted = match (model_info.appearance, model::painted_model_stem(&model)) {
            (Some(appearance), Some(stem)) => {
                match icon_paint(
                    &mut loader,
                    painting.as_deref_mut(),
                    stem,
                    appearance,
                    deadline,
                ) {
                    Some(handle) => Some(handle),
                    // The paint is under way: the icon waits, and keeps its
                    // attempts.
                    None => {
                        index += 1;
                        continue;
                    }
                }
            }
            _ => None,
        };
        let variant = icons.pending[index].variant;
        let turned = framing(variant, false).turned;
        let hide_star = model_info.flags & 0x5f == 9;
        let scene = IconScene::from_model(
            &model,
            &meshes,
            &materials,
            hide_star,
            turned,
            painted.as_ref(),
        );
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

/// The painted material of `stem` with `appearance` for an icon, once it
/// is ready; `None` while its textures decode or its paint job runs. The
/// job goes to the async compute pool when there is one (and painting is
/// tracked), else it is painted on the calling thread.
fn icon_paint(
    loader: &mut ModelLoader<'_>,
    painting: Option<&mut IconPainting>,
    stem: String,
    appearance: Appearance,
    deadline: std::time::Instant,
) -> Option<Handle<exa_render::material::ExaMaterial>> {
    let key = (stem, appearance);
    if painting
        .as_ref()
        .is_some_and(|painting| painting.tasks.contains_key(&key))
    {
        return None;
    }
    if !loader.warm_paint(&key.0, &appearance, deadline) {
        return None;
    }
    match loader.paint_job(&key.0, &appearance) {
        Paint::Ready(handle) => Some(handle),
        Paint::Job(job) => match (painting, AsyncComputeTaskPool::try_get()) {
            (Some(painting), Some(pool)) => {
                painting
                    .tasks
                    .insert(key, pool.spawn(async move { job.paint() }));
                None
            }
            _ => Some(loader.painted_material(job.paint())),
        },
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
    materials: Option<Res<Assets<exa_render::material::ExaMaterial>>>,
) {
    // Nothing to start: no texture scan and no change mark on the icons.
    if !icons.pending.iter().any(|pending| pending.scene.is_some()) {
        return;
    }
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
    let mut budget = start_budget(mode.gpu, jobs.jobs.len());
    let mut index = 0;
    while index < icons.pending.len() && budget > 0 {
        let Some(scene) = icons.pending[index].scene.as_ref() else {
            index += 1;
            continue;
        };
        // The GPU camera draws what its materials bind now: an icon waits
        // until its textures are in, so it is not baked over the
        // placeholder.
        if mode.gpu
            && icons.pending[index].waited < TEXTURE_WAIT
            && let (Some(materials), Some(images)) = (materials.as_deref(), images.as_deref())
            && !scene_textures_ready(scene, materials, images)
        {
            icons.pending[index].waited += 1;
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

/// How many prepared icons a frame starts. The CPU paint takes up to
/// [`ICONS_PER_FRAME`]; a GPU job puts its scene, camera and sun on the one
/// icon layer, where every camera sees every scene and every sun, so only
/// one GPU job is in flight at a time (two at once rendered the second
/// item framed on the first one's model, lit by two suns). The light
/// layers start right after the icon layer, so the jobs cannot take a
/// layer each.
pub fn start_budget(gpu: bool, in_flight: usize) -> usize {
    match (gpu, in_flight) {
        (false, _) => ICONS_PER_FRAME,
        (true, 0) => 1,
        (true, _) => 0,
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
            Offscreen,
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
            Offscreen,
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
            // The shadow linker would move a caster onto the world's layer.
            NotShadowCaster,
            NotShadowReceiver,
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
    if jobs.jobs.is_empty() {
        return;
    }
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
    mut images: Option<ResMut<Assets<Image>>>,
) {
    if jobs.jobs.iter().all(|job| job.frames < ICON_JOB_TIMEOUT) {
        return;
    }
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
        for entity in [job.camera, job.light, job.root]
            .into_iter()
            .chain(job.readback)
        {
            commands.entity(entity).try_despawn();
        }
        if let Some(images) = images.as_deref_mut() {
            images.remove(&job.image);
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
    let rendered = icon_from_readback(
        &trigger.event().data,
        job.buffer,
        job.icon,
        job.screen_pass,
        job.downsample,
    )
    .filter(IconPixels::any_alpha)
    .unwrap_or_else(|| job.scene.paint(&job.paint_camera, job.sun, job.icon));
    let mut icon = rendered;
    if job.halo {
        exa_items::item_gui::halo(
            &mut icon.pixels,
            icon.width as usize,
            icon.height as usize,
            job.halo_highlight,
        );
    }
    let applied =
        icons.apply_rendered(job.id, job.key, icon, job.content_width, job.content_height);
    debug!(
        "icon: {:#x} key {:#x} rendered on the GPU, applied {applied}",
        job.id, job.key
    );
    for entity in [job.camera, job.light, job.root]
        .into_iter()
        .chain(job.readback)
    {
        commands.entity(entity).try_despawn();
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
    fn one_gpu_icon_is_in_flight_at_a_time() {
        assert_eq!(start_budget(true, 0), 1);
        assert_eq!(start_budget(true, 1), 0);
        assert_eq!(start_budget(false, 3), ICONS_PER_FRAME);
    }

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

    /// A shipped apparel item's icon scene takes the material its
    /// appearance paints for every run-time (`&`) part, not the unpainted
    /// shade-map stand-in that rendered the chest's clothes white (skipped
    /// without an installation).
    #[test]
    fn a_shipped_apparel_icon_is_painted_from_its_appearance() {
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
        let catalogue = ItemCatalogue::of_database(&db);
        let items = exa_formats::objdb::items(&db);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<exa_render::material::ExaMaterial>()
            .init_asset::<Image>()
            .init_resource::<ModelCache>()
            .init_resource::<PanelIcons>()
            .init_resource::<IconPainting>()
            .insert_resource(catalogue.clone())
            .insert_resource(Game(std::sync::Arc::new(data)))
            .add_systems(Update, prepare_icon_scenes);
        // The first apparel record whose ground model has a run-time part.
        let mut found = None;
        for item in items.iter().filter(|item| {
            item.appearance.is_some() && !item.model.is_empty() && !item.model.starts_with('@')
        }) {
            let world = app.world_mut();
            let stem = world.resource_scope(|world, game: Mut<Game>| {
                world.resource_scope(|world, mut cache: Mut<ModelCache>| {
                    world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                        world.resource_scope(
                            |world, mut materials: Mut<Assets<exa_render::material::ExaMaterial>>| {
                                let mut images = world.resource_mut::<Assets<Image>>();
                                let mut loader = ModelLoader {
                                    game: &game.0,
                                    cache: &mut cache,
                                    meshes: &mut meshes,
                                    materials: &mut materials,
                                    images: &mut images,
                                    bc_supported: false,
                                };
                                // The icon's own lookup: the objects, then the
                                // object library.
                                let pack = [PackKind::Objects, PackKind::Objlib]
                                    .into_iter()
                                    .find(|&pack| {
                                        loader
                                            .game
                                            .pack(pack)
                                            .is_some_and(|a| a.get(&item.model).is_some())
                                    })?;
                                let model = loader.model(pack, &item.model)?;
                                model::painted_model_stem(&model)
                            },
                        )
                    })
                })
            });
            if stem.is_some() {
                found = Some(item.clone());
                break;
            }
        }
        let item = found.expect("the shipped database has an apparel item with a run-time part");
        let mut icons = app.world_mut().resource_mut::<PanelIcons>();
        let _ = icons.reserve(item.id, 0, Some(&catalogue));
        // The paint runs on the async pool; the icon waits for it.
        for _ in 0..200 {
            app.update();
            if app.world().resource::<PanelIcons>().pending[0]
                .scene
                .is_some()
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let icons = app.world().resource::<PanelIcons>();
        let scene = icons.pending[0]
            .scene
            .as_ref()
            .unwrap_or_else(|| panic!("{} never got its scene", item.name));
        let materials = app
            .world()
            .resource::<Assets<exa_render::material::ExaMaterial>>();
        let painted = scene.parts.iter().any(|part| {
            materials.get(&part.material).is_some_and(|m| {
                m.params.flags & exa_render::material::flags::PACKED_SPECULAR != 0
                    && m.diffuse_map.is_some()
            })
        });
        assert!(painted, "{} keeps its unpainted materials", item.name);
        assert!(app.world().resource::<IconPainting>().tasks.is_empty());
    }

    /// A GPU icon whose model keeps its own (non-apparel) materials waits
    /// while a texture they bind is still the streamed load's 1 × 1
    /// placeholder, and starts once the upload has replaced it under the
    /// same handle; the scene keeps the loaded material, no paint.
    #[test]
    fn a_gpu_icon_waits_for_its_materials_textures() {
        use exa_render::material::ExaMaterial;
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Image>()
            .init_asset::<Mesh>()
            .init_asset::<ExaMaterial>()
            .init_resource::<PanelIcons>()
            .init_resource::<IconGpuJobs>()
            .insert_resource(IconRenderMode { gpu: true })
            .add_systems(Update, paint_icon_jobs);
        let placeholder = Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[128, 128, 128, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        let texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(placeholder);
        let material = app
            .world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .add(ExaMaterial {
                diffuse_map: Some(texture.clone()),
                ..Default::default()
            });
        let scene = IconScene {
            parts: vec![item_icon::IconPart {
                mesh: Handle::default(),
                material: material.clone(),
                transform: Mat4::IDENTITY,
            }],
            ..IconScene::synthetic_quad()
        };
        {
            let mut icons = app.world_mut().resource_mut::<PanelIcons>();
            let _ = icons.reserve(0x54, 0, None);
            icons.pending[0].scene = Some(scene);
        }
        for frame in 1..=3 {
            app.update();
            let icons = app.world().resource::<PanelIcons>();
            assert_eq!(icons.pending[0].waited, frame);
            assert!(app.world().resource::<IconGpuJobs>().jobs.is_empty());
        }
        // The streamed upload writes the real pixels under the same handle.
        let loaded = Image::new_fill(
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[90, 60, 30, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        let _ = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(texture.id(), loaded);
        app.update();
        assert!(app.world().resource::<PanelIcons>().pending.is_empty());
        let jobs = app.world().resource::<IconGpuJobs>();
        assert_eq!(jobs.jobs.len(), 1);
        assert_eq!(jobs.jobs[0].scene.parts[0].material, material);
        // A placeholder that never goes renders after the wait limit.
        const { assert!(TEXTURE_WAIT > 3) };
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
    fn the_icon_camera_and_scene_stay_off_the_world() {
        // The off-screen layer is below every light's shadow layer, so the
        // light-layer pass never hands it to a scene camera.
        const {
            assert!(ICON_LAYER < exa_world::shadow_links::FIRST_LIGHT_LAYER);
            assert!(ICON_LAYER != 0);
        }
        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        let scene = IconScene {
            parts: vec![item_icon::IconPart {
                mesh: Handle::default(),
                material: Handle::default(),
                transform: Mat4::IDENTITY,
            }],
            ..IconScene::synthetic_quad()
        };
        let (view, plan) = item_icon::scene_plan(0, false, 0, scene.bounds, 160, true);
        let pending = PendingIcon {
            id: 3,
            key: 0,
            variant: 0,
            entry: 0,
            model_flags: 0,
            scene: None,
            attempts: 0,
            waited: 0,
        };
        let job = world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let mut commands = Commands::new(&mut queue, world);
            let job = spawn_gpu_icon(
                &mut commands,
                &mut images,
                &pending,
                &scene,
                &view,
                &plan,
                160,
            )
            .expect("a job");
            queue.apply(world);
            job
        });
        let layer = RenderLayers::layer(ICON_LAYER);
        for entity in [job.camera, job.light] {
            assert!(world.get::<Offscreen>(entity).is_some());
            assert_eq!(world.get::<RenderLayers>(entity), Some(&layer));
        }
        let mut parts = world.query_filtered::<&RenderLayers, With<Mesh3d>>();
        let parts: Vec<_> = parts.iter(&world).collect();
        assert_eq!(parts, vec![&layer]);
        let mut casters = world.query_filtered::<(), (With<Mesh3d>, Without<NotShadowCaster>)>();
        assert_eq!(casters.iter(&world).count(), 0);
        assert_eq!(world.resource::<Assets<Image>>().len(), 1);
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
