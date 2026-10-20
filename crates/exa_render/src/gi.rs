//! The original's voxel global illumination: a block of 20 cm voxels where
//! light from the level's lights spreads through the open cells, one step
//! per frame, and surfaces take their ambient from it. The occupancy comes
//! from the world (`exa_world::gi_volume`); this module owns the volume
//! textures, the propagation pass (`gi.wesl`) and the lights it injects:
//! point lights, and voxel lights (glowing items), which add their light
//! straight into the volume around themselves.
//! Rules: `docs/RENDERING.md` (Voxel GI).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::core_pipeline::Core3dSystems;
use bevy::core_pipeline::schedule::Core3d;
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{
    sampler, texture_3d, texture_storage_3d, uniform_buffer,
};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedComputePipelineId,
    ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, FilterMode, PipelineCache, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, StorageTextureAccess,
    TexelCopyTextureInfo, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::view::ExtractedView;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};

use crate::material::{ExaMaterial, flags, linear_rgb};

pub struct GiPlugin;

impl Plugin for GiPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "gi.wesl");
        let warm_up = GiWarmUp::default();
        let report = WarmUpReport(warm_up.report.clone());
        app.init_resource::<GiLights>()
            .insert_resource(warm_up)
            .add_plugins((
                ExtractResourcePlugin::<GiTextures>::default(),
                ExtractResourcePlugin::<GiPendingTextures>::default(),
                ExtractResourcePlugin::<GiLights>::default(),
            ))
            .add_systems(
                PostUpdate,
                (
                    create_textures,
                    swap_warm_textures,
                    gather_lights,
                    give_materials_gi,
                )
                    .chain()
                    .in_set(GiSystems),
            );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<GiOwed>()
            .insert_resource(report)
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Core3d, propagate.in_set(Core3dSystems::Prepass));
    }
}

/// The main-world GI systems: they make the textures of a new [`GiVolume`]
/// and gather the lights. Whatever inserts a volume runs before them, so the
/// textures and [`GiWarmUp`] follow it within the frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GiSystems;

/// How far the warm-up of the newest volume has run, in the main world (a
/// loading screen waits for it). The steps run in the render world, which
/// reports each frame's count through a counter both worlds hold: the
/// volume's serial in the high half, the steps done in the low half. Nothing
/// is read back from the GPU; a step counts once it has been encoded.
#[derive(Resource, Default)]
pub struct GiWarmUp {
    report: Arc<AtomicU64>,
    /// The newest volume's serial and its warm-up length.
    serial: u32,
    steps: u32,
}

impl GiWarmUp {
    /// The share of the newest volume's warm-up that has run (0–1); 0
    /// before there is a volume.
    pub fn progress(&self) -> f32 {
        if self.serial == 0 {
            return 0.0;
        }
        if self.steps == 0 {
            return 1.0;
        }
        let report = self.report.load(Ordering::Acquire);
        if (report >> 32) as u32 != self.serial {
            return 0.0;
        }
        (report as u32).min(self.steps) as f32 / self.steps as f32
    }

    /// Whether the newest volume's warm-up has run in full.
    pub fn finished(&self) -> bool {
        self.progress() >= 1.0
    }
}

/// The render world's side of [`GiWarmUp`].
#[derive(Resource)]
struct WarmUpReport(Arc<AtomicU64>);

/// The occupancy of a GI block, as the world built it: cells X fastest,
/// then Z, then Y (0..255).
#[derive(Resource, Clone)]
pub struct GiVolume {
    /// Lowest corner, metres.
    pub origin: Vec3,
    /// Voxels along X, Z and Y.
    pub size: UVec3,
    /// Voxel edge, metres.
    pub voxel: f32,
    pub cells: Arc<Vec<u8>>,
    /// The lowest slice light propagates in, per column (X fastest, then
    /// Z); 255 where it never does.
    pub floors: Arc<Vec<u8>>,
    /// The faint light every open voxel gains per step (level record 0x30
    /// × 0.001).
    pub base_ambient: f32,
    /// Steps run at once when the volume is created.
    pub warm_up: u32,
}

/// The GI textures and where they sit.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiTextures {
    pub occupancy: Handle<Image>,
    floors: Handle<Image>,
    pub light: Handle<Image>,
    scratch: Handle<Image>,
    pub origin: Vec3,
    pub extent: Vec3,
    /// Voxel edge, metres, and the texture's size (X, Z, Y).
    voxel: f32,
    dims: UVec3,
    base_ambient: f32,
    /// Steps run at once when the volume is created.
    warm_up: u32,
    /// Which volume these are (counting from 1), so the render world starts
    /// each volume's warm-up once.
    serial: u32,
}

/// The next block's textures, warming while the previous block stays
/// active. A block rebuilt as the camera moves starts black; swapping it in
/// before its warm-up has run leaves hard black patches and halos on the
/// floor. The old block stays bound until the new one is warm.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiPendingTextures(pub GiTextures);

/// A light's share of the GI injection (its record's multiplier); lights
/// without one inject in full.
#[derive(Component, Clone, Copy, Debug)]
pub struct GiMultiplier(pub f32);

/// A voxel light (the original's `0x3D0C` node: a salve's, a power
/// crystal's or embers' glow). Every step it adds its colour straight into
/// the voxels around it; it never lights a surface directly.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct VoxelLight {
    /// lin(colour)·√intensity·3.
    pub colour: Vec3,
}

impl VoxelLight {
    /// The light of a node's sRGB colour and intensity: the intensity
    /// counts by its square root, ×3 in the original's linear workflow.
    pub fn new(srgb: [f32; 3], intensity: f32) -> Self {
        Self {
            colour: linear_rgb(srgb) * intensity.max(0.0).sqrt() * VOXEL_LIGHT_GAIN,
        }
    }
}

/// The lights injected into the volume this frame.
#[derive(Resource, Clone, Default, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiLights {
    lights: Vec<GiLight>,
    voxel_lights: Vec<GiVoxelLight>,
}

#[derive(Clone, Copy, Default, ShaderType)]
struct GiLight {
    position_radius: Vec4,
    colour: Vec4,
    /// Vertical shaping (y, z), as the surface shader reads it.
    shaping: Vec4,
}

const MAX_LIGHTS: usize = 32;

/// A voxel light as the pass reads it.
#[derive(Clone, Copy, Default, ShaderType)]
struct GiVoxelLight {
    /// The light in voxels from the block's corner (texture axes X, Z, Y).
    position: Vec4,
    /// Per-step colour: [`VoxelLight::colour`] × 0.1.
    colour: Vec4,
    /// The voxels it reaches: from `lo` up to but not including `hi`.
    lo: UVec4,
    hi: UVec4,
}

/// Voxel lights injected into a block at once; the corpus has at most a
/// handful in a level.
const MAX_VOXEL_LIGHTS: usize = 64;

#[derive(Clone, ShaderType)]
struct GiParams {
    origin: Vec4,
    extent: Vec4,
    lights: [GiLight; MAX_LIGHTS],
    /// x = the number of voxel lights.
    voxel_count: UVec4,
    voxel_lights: [GiVoxelLight; MAX_VOXEL_LIGHTS],
}

/// Lumens per unit of the original's light intensity (see `exa_world`).
const LUMENS_PER_INTENSITY: f32 = 8.0;
/// Light ranges are 1.25 radii; the volume falls off over one radius.
const REACH: f32 = 1.25;
/// The per-step injection of a light: 0.15 × 0.01 of its colour.
const INJECTION: f32 = 0.0015;
/// A voxel light's colour per unit of √intensity (the original's linear
/// workflow; 0.6 on its legacy path).
const VOXEL_LIGHT_GAIN: f32 = 3.0;
/// The share of a voxel light's colour added per step.
const VOXEL_LIGHT_SHARE: f32 = 0.1;
/// The columns a voxel light reaches on each side of the voxel corner
/// nearest it, and the slices below it (one more above).
const VOXEL_LIGHT_REACH: i32 = 3;
/// Columns at each side of a block a voxel light never reaches.
const VOXEL_LIGHT_MARGIN: i32 = 8;

/// The voxels a voxel light at `at` (in voxels from the block's corner,
/// texture axes X, Z, Y) adds to, as the original draws it: 6 × 6 columns
/// around the voxel corner nearest it, outside the block's 8-column
/// margins, and the slices from 3 below that corner to 3 above (fewer when
/// it is within 6 slices of the bottom; none at or below it). `None` when
/// that leaves nothing.
pub fn voxel_light_box(at: Vec3, dims: UVec3) -> Option<(UVec3, UVec3)> {
    if !at.is_finite() {
        return None;
    }
    let corner = at.round().as_ivec3();
    let reach = VOXEL_LIGHT_REACH;
    let columns = |c: i32, size: u32| {
        let far = size as i32 - VOXEL_LIGHT_MARGIN;
        ((c - reach).max(VOXEL_LIGHT_MARGIN), (c + reach).min(far))
    };
    let (x0, x1) = columns(corner.x, dims.x);
    let (y0, y1) = columns(corner.y, dims.y);
    let k = corner.z;
    if x0 >= x1 || y0 >= y1 || k <= 0 {
        return None;
    }
    // 2·reach + 1 slices from k − reach, k + 1 of them near the bottom.
    let first = k - reach;
    let count = (k + 1).min(2 * reach + 1);
    let (z0, z1) = (first.max(0), (first + count).min(dims.z as i32));
    if z0 >= z1 {
        return None;
    }
    Some((
        UVec3::new(x0 as u32, y0 as u32, z0 as u32),
        UVec3::new(x1 as u32, y1 as u32, z1 as u32),
    ))
}

/// What a voxel light adds to a voxel per step, per unit of its per-step
/// colour (`gi.wesl` does the same): `offset` runs from the light to the
/// voxel's centre in voxels (texture axes), `blocked` is min(20·(1 −
/// o(voxel)·o(midpoint)), 1) with o = 1 − occupancy. The light falls off
/// as max(1 − 5000·d², 0)⁴ with d in the block's texture units (W = 256
/// voxels across, the vertical counted the same way), reaching nothing at
/// 3.62 voxels; where geometry blocks it, only about 1.2 voxels around the
/// light are lit.
pub fn voxel_light_weight(offset: Vec3, dims: UVec3, blocked: f32) -> f32 {
    let w = dims.x.max(1) as f32;
    let h = dims.y.max(1) as f32;
    let d = Vec3::new(offset.x / w, offset.y / h, offset.z / w);
    let atn = (1.0 - 5000.0 * d.length_squared()).max(0.0).powi(4);
    if atn <= 0.0 {
        return 0.0;
    }
    let near = (2.0 - 0.75 / (atn * atn)).max(0.0);
    atn * 0.25 * (1.0 - blocked * (1.0 - near))
}

fn volume_image(size: UVec3, format: TextureFormat, data: Vec<u8>, storage: bool) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: size.z,
        },
        TextureDimension::D3,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::COPY_SRC
        | if storage {
            TextureUsages::STORAGE_BINDING
        } else {
            TextureUsages::empty()
        };
    image
}

/// The light volume: stored as plain 8-bit (the pass writes a plain
/// scratch copy, copied over each step), and sampled through an sRGB view,
/// decoded before filtering as the original's sRGB texture is.
fn light_image(size: UVec3, data: Vec<u8>) -> Image {
    let mut image = volume_image(size, TextureFormat::Rgba8Unorm, data, false);
    image.texture_descriptor.view_formats = &[TextureFormat::Rgba8UnormSrgb];
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        format: Some(TextureFormat::Rgba8UnormSrgb),
        ..default()
    });
    image
}

fn create_textures(
    mut commands: Commands,
    volume: Option<Res<GiVolume>>,
    active: Option<Res<GiTextures>>,
    mut images: ResMut<Assets<Image>>,
    mut warm_up: ResMut<GiWarmUp>,
) {
    let Some(volume) = volume else { return };
    if !volume.is_changed() {
        return;
    }
    // Texture axes: X, Z, Y.
    let size = UVec3::new(volume.size.x, volume.size.y, volume.size.z);
    let voxels = (size.x * size.y * size.z) as usize;
    let occupancy = images.add(volume_image(
        size,
        TextureFormat::R8Unorm,
        volume.cells.as_ref().clone(),
        false,
    ));
    let floors = images.add(volume_image(
        UVec3::new(size.x, size.y, 1),
        TextureFormat::R8Unorm,
        volume.floors.as_ref().clone(),
        false,
    ));
    let empty = vec![0u8; voxels * 4];
    let light = images.add(light_image(size, empty.clone()));
    let scratch = images.add(volume_image(size, TextureFormat::Rgba8Unorm, empty, true));
    let extent = Vec3::new(size.x as f32, size.z as f32, size.y as f32) * volume.voxel;
    warm_up.serial += 1;
    warm_up.steps = volume.warm_up;
    let textures = GiTextures {
        occupancy,
        floors,
        light,
        scratch,
        origin: volume.origin,
        extent,
        voxel: volume.voxel,
        dims: size,
        base_ambient: volume.base_ambient,
        warm_up: volume.warm_up,
        serial: warm_up.serial,
    };
    // The first block is shown while it warms (the loading screen waits);
    // a later block warms behind the old one.
    if active.is_some() {
        commands.insert_resource(GiPendingTextures(textures));
    } else {
        commands.insert_resource(textures);
    }
}

/// Swaps in a rebuilt block once its warm-up has run in full. The old block
/// stays bound until then, so the floor never shows a cold block's black
/// patches or propagation halo.
fn swap_warm_textures(
    mut commands: Commands,
    pending: Option<Res<GiPendingTextures>>,
    warm_up: Res<GiWarmUp>,
) {
    let Some(pending) = pending else { return };
    if !warm_up.finished() {
        return;
    }
    info!(
        "GI block warm: origin {:.2}, serial {}",
        pending.0.origin, pending.0.serial
    );
    commands.insert_resource(pending.0.clone());
    commands.remove_resource::<GiPendingTextures>();
}

fn gather_lights(
    mut gi: ResMut<GiLights>,
    lights: Query<(
        &PointLight,
        &GlobalTransform,
        Option<&GiMultiplier>,
        Option<&crate::lights::LightShaping>,
    )>,
    voxel_lights: Query<(&VoxelLight, &GlobalTransform)>,
    textures: Option<Res<GiTextures>>,
) {
    gi.lights.clear();
    gi.voxel_lights.clear();
    let Some(textures) = textures else {
        return;
    };
    gather_voxel_lights(&mut gi.voxel_lights, &textures, voxel_lights.iter());
    // The lights whose reach overlaps the block, nearest its centre first
    // (the original injects the lights of each sector group).
    let (lo, hi) = (textures.origin, textures.origin + textures.extent);
    let centre = (lo + hi) * 0.5;
    let mut near: Vec<_> = lights
        .iter()
        .filter(|l| l.2.is_none_or(|m| m.0 > 0.0))
        .filter(|(light, at, _, _)| {
            let p = at.translation();
            p.clamp(lo, hi).distance(p) < light.range / REACH
        })
        .collect();
    near.sort_by(|a, b| {
        a.1.translation()
            .distance_squared(centre)
            .total_cmp(&b.1.translation().distance_squared(centre))
    });
    for (light, at, share, shaping) in near.into_iter().take(MAX_LIGHTS) {
        let c = light.color.to_linear();
        let intensity = light.intensity / LUMENS_PER_INTENSITY * share.map_or(1.0, |m| m.0);
        gi.lights.push(GiLight {
            position_radius: at.translation().extend(light.range / REACH),
            colour: (Vec3::new(c.red, c.green, c.blue) * intensity * INJECTION).extend(0.0),
            shaping: shaping.map_or(Vec2::ONE, |s| s.0).extend(0.0).extend(0.0),
        });
    }
}

/// The voxel lights that reach into the block, nearest its centre first (the
/// original injects every sector's voxel lights).
fn gather_voxel_lights<'a>(
    out: &mut Vec<GiVoxelLight>,
    textures: &GiTextures,
    lights: impl Iterator<Item = (&'a VoxelLight, &'a GlobalTransform)>,
) {
    let centre = textures.dims.as_vec3() * 0.5;
    let mut near: Vec<(f32, GiVoxelLight)> = lights
        .filter(|(light, _)| light.colour.max_element() > 0.0)
        .filter_map(|(light, at)| {
            // World X, Y, Z to texture X, Z, Y, in voxels.
            let p = (at.translation() - textures.origin) / textures.voxel;
            let voxels = Vec3::new(p.x, p.z, p.y);
            let (lo, hi) = voxel_light_box(voxels, textures.dims)?;
            Some((
                voxels.distance_squared(centre),
                GiVoxelLight {
                    position: voxels.extend(0.0),
                    colour: (light.colour * VOXEL_LIGHT_SHARE).extend(0.0),
                    lo: lo.extend(0),
                    hi: hi.extend(0),
                },
            ))
        })
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.extend(near.into_iter().take(MAX_VOXEL_LIGHTS).map(|(_, l)| l));
}

/// Points every surface material at the volume once it exists (and new
/// materials as they come).
fn give_materials_gi(
    textures: Option<Res<GiTextures>>,
    mut materials: ResMut<Assets<ExaMaterial>>,
    mut events: MessageReader<AssetEvent<ExaMaterial>>,
) {
    let Some(textures) = textures else {
        events.clear();
        return;
    };
    let added: Vec<AssetId<ExaMaterial>> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Added { id } => Some(*id),
            _ => None,
        })
        .collect();
    let ids: Vec<AssetId<ExaMaterial>> = if textures.is_changed() {
        materials.ids().collect()
    } else {
        added
    };
    for id in ids {
        let Some(mut m) = materials.get_mut(id) else {
            continue;
        };
        if m.params.flags & flags::LIT == 0 {
            continue;
        }
        m.gi_volume = Some(textures.light.clone());
        m.params.gi_origin = textures.origin.extend(1.0);
        m.params.gi_extent = textures.extent.extend(0.0);
        m.params.flags |= flags::GI;
        if std::env::var_os("EXA_GI_DEBUG").is_some() {
            m.params.flags |= flags::DEBUG_GI;
        }
    }
}

#[derive(Resource)]
struct GiPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline: CachedComputePipelineId,
}

fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    assets: Res<AssetServer>,
    cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "exa_gi",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_3d(TextureSampleType::Float { filterable: true }),
                texture_3d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_storage_3d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                uniform_buffer::<GiParams>(false),
                texture_3d(TextureSampleType::Float { filterable: true }),
            ),
        ),
    );
    let pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("exa_gi_propagate".into()),
        layout: vec![layout.clone()],
        shader: assets
            .load(AssetPath::from_path_buf(embedded_path!("gi.wesl")).with_source("embedded")),
        entry_point: Some("propagate".into()),
        ..default()
    });
    commands.insert_resource(GiPipeline {
        layout,
        sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        pipeline,
    });
}

/// The pass's bind group and buffer.
#[derive(Resource)]
struct GiBindGroup {
    bind_group: bevy::render::render_resource::BindGroup,
    _params: UniformBuffer<GiParams>,
}

/// The warm-up of the volume being lit: its serial, its length and the
/// steps not yet run (kept until the pipeline is ready).
#[derive(Resource, Default)]
struct GiOwed {
    serial: u32,
    total: u32,
    left: u32,
}

/// Warm-up steps done in one frame at most; the rest follow in later frames.
const MAX_STEPS_PER_FRAME: u32 = 16;

#[allow(clippy::too_many_arguments)]
fn prepare_bind_group(
    mut commands: Commands,
    textures: Option<Res<GiTextures>>,
    pending: Option<Res<GiPendingTextures>>,
    lights: Res<GiLights>,
    pipeline: Option<Res<GiPipeline>>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut owed: ResMut<GiOwed>,
) {
    let textures = pending.as_ref().map(|p| &p.0).or(textures.as_deref());
    let (Some(textures), Some(pipeline)) = (textures, pipeline) else {
        commands.remove_resource::<GiBindGroup>();
        return;
    };
    let (Some(light), Some(occupancy), Some(scratch), Some(floors)) = (
        images.get(&textures.light),
        images.get(&textures.occupancy),
        images.get(&textures.scratch),
        images.get(&textures.floors),
    ) else {
        return;
    };
    let mut entries = [GiLight::default(); MAX_LIGHTS];
    for (slot, l) in entries.iter_mut().zip(&lights.lights) {
        *slot = *l;
    }
    let mut voxel_entries = [GiVoxelLight::default(); MAX_VOXEL_LIGHTS];
    for (slot, l) in voxel_entries.iter_mut().zip(&lights.voxel_lights) {
        *slot = *l;
    }
    let mut params = UniformBuffer::from(GiParams {
        origin: textures.origin.extend(textures.base_ambient),
        extent: textures
            .extent
            .extend(lights.lights.len().min(MAX_LIGHTS) as f32),
        lights: entries,
        voxel_count: UVec4::new(
            lights.voxel_lights.len().min(MAX_VOXEL_LIGHTS) as u32,
            0,
            0,
            0,
        ),
        voxel_lights: voxel_entries,
    });
    params.write_buffer(&device, &queue);
    let Some(binding) = params.binding() else {
        return;
    };
    let bind_group = device.create_bind_group(
        "exa_gi",
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((
            &light.texture_view,
            &occupancy.texture_view,
            &pipeline.sampler,
            &scratch.texture_view,
            binding,
            &floors.texture_view,
        )),
    );
    if owed.serial != textures.serial {
        *owed = GiOwed {
            serial: textures.serial,
            total: textures.warm_up,
            left: textures.warm_up,
        };
    }
    commands.insert_resource(GiBindGroup {
        bind_group,
        _params: params,
    });
}

#[allow(clippy::too_many_arguments)]
fn propagate(
    view: ViewQuery<(&ExtractedView, Has<crate::supersample::DisplayCamera>)>,
    textures: Option<Res<GiTextures>>,
    pending: Option<Res<GiPendingTextures>>,
    bind_group: Option<Res<GiBindGroup>>,
    pipeline: Option<Res<GiPipeline>>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    mut owed: ResMut<GiOwed>,
    report: Res<WarmUpReport>,
    mut ctx: RenderContext,
) {
    // Once per frame, for the scene view (not a supersampling display).
    if view.into_inner().1 {
        return;
    }
    let textures = pending.as_ref().map(|p| &p.0).or(textures.as_deref());
    let (Some(textures), Some(bind_group), Some(pipeline)) = (textures, bind_group, pipeline)
    else {
        return;
    };
    let (Some(compute), Some(light), Some(scratch)) = (
        cache.get_compute_pipeline(pipeline.pipeline),
        images.get(&textures.light),
        images.get(&textures.scratch),
    ) else {
        return;
    };
    let size = light.texture.size();
    // The warm-up runs a few steps a frame (all of it in one frame stalled
    // the window for seconds); then one step per frame.
    let steps = owed.left.clamp(1, MAX_STEPS_PER_FRAME);
    owed.left -= steps.min(owed.left);
    report.0.store(
        (u64::from(owed.serial) << 32) | u64::from(owed.total - owed.left),
        Ordering::Release,
    );
    // A GPU timing span for the frame profiler, when render diagnostics are
    // on.
    let diagnostics = ctx.diagnostic_recorder();
    let span = diagnostics
        .as_deref()
        .map(|d| d.time_span(ctx.command_encoder(), "exa_gi"));
    for _ in 0..steps {
        let encoder = ctx.command_encoder();
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("exa_gi_propagate"),
                timestamp_writes: None,
            });
            pass.set_pipeline(compute);
            pass.set_bind_group(0, &bind_group.bind_group, &[]);
            pass.dispatch_workgroups(
                size.width.div_ceil(4),
                size.height.div_ceil(4),
                size.depth_or_array_layers.div_ceil(4),
            );
        }
        // The new step becomes the light the surfaces and the next step read.
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo {
                texture: &scratch.texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            },
            TexelCopyTextureInfo {
                texture: &light.texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            },
            size,
        );
    }
    if let Some(span) = span {
        span.end(ctx.command_encoder());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: UVec3 = UVec3::new(256, 256, 64);

    fn fake_textures(serial: u32) -> GiTextures {
        GiTextures {
            occupancy: Handle::default(),
            floors: Handle::default(),
            light: Handle::default(),
            scratch: Handle::default(),
            origin: Vec3::ZERO,
            extent: Vec3::ONE,
            voxel: 0.2,
            dims: UVec3::ONE,
            base_ambient: 0.0,
            warm_up: 10,
            serial,
        }
    }

    fn block_volume(origin: Vec3) -> GiVolume {
        GiVolume {
            origin,
            size: UVec3::new(2, 2, 2),
            voxel: 0.2,
            cells: Arc::new(vec![0; 8]),
            floors: Arc::new(vec![0; 4]),
            base_ambient: 0.0,
            warm_up: 10,
        }
    }

    #[test]
    fn a_rebuilt_block_waits_for_its_warm_up() {
        let warm = GiWarmUp {
            serial: 1,
            steps: 10,
            ..default()
        };
        let mut app = App::new();
        app.insert_resource(Assets::<Image>::default())
            .insert_resource(warm)
            .insert_resource(fake_textures(1))
            .insert_resource(block_volume(Vec3::ZERO))
            .add_systems(Update, (create_textures, swap_warm_textures).chain());
        app.update();
        // The old block stays bound; the new one only warms behind it.
        assert_eq!(app.world().resource::<GiTextures>().serial, 1);
        assert_eq!(app.world().resource::<GiPendingTextures>().0.serial, 2);
        app.update();
        assert_eq!(app.world().resource::<GiTextures>().serial, 1);

        // Once the render world has reported every step, it swaps.
        let warm = app.world().resource::<GiWarmUp>();
        warm.report.store(
            (u64::from(warm.serial) << 32) | u64::from(warm.steps),
            Ordering::Release,
        );
        app.update();
        assert_eq!(app.world().resource::<GiTextures>().serial, 2);
        assert!(!app.world().contains_resource::<GiPendingTextures>());
    }

    #[test]
    fn the_first_block_is_used_while_it_warms() {
        let mut app = App::new();
        app.insert_resource(Assets::<Image>::default())
            .init_resource::<GiWarmUp>()
            .insert_resource(block_volume(Vec3::ZERO))
            .add_systems(Update, (create_textures, swap_warm_textures).chain());
        app.update();
        assert!(app.world().contains_resource::<GiTextures>());
        assert!(!app.world().contains_resource::<GiPendingTextures>());
    }

    #[test]
    fn salve_voxel_light_colour() {
        // The menu's Healing Salve: (0.2157, 0.6784, 1.0), intensity 16.
        let light = VoxelLight::new([0.2157, 0.6784, 1.0], 16.0);
        let expected = Vec3::new(0.458, 5.01, 12.0);
        assert!((light.colour - expected).abs().max_element() < 0.01);
        // Its per-step colour at its own voxel, in open space.
        let step = light.colour * VOXEL_LIGHT_SHARE * voxel_light_weight(Vec3::ZERO, BLOCK, 0.0);
        assert!((step - Vec3::new(0.0115, 0.125, 0.30)).abs().max_element() < 0.001);
        // A spent salve (condition share 0.0044) is about 15 times dimmer.
        let spent = VoxelLight::new([0.2157, 0.6784, 1.0], 16.0 * 0.0044);
        assert!((spent.colour.z - 12.0 * 0.0044f32.sqrt()).abs() < 1e-3);
        assert_eq!(VoxelLight::new([1.0; 3], -1.0).colour, Vec3::ZERO);
    }

    #[test]
    fn voxel_light_falls_off_within_four_voxels() {
        let w = |n: f32, blocked: f32| voxel_light_weight(Vec3::new(n, 0.0, 0.0), BLOCK, blocked);
        assert!((w(0.0, 0.0) - 0.25).abs() < 1e-6);
        assert!(w(3.6, 0.0) > 0.0);
        assert_eq!(w(3.63, 0.0), 0.0);
        // Vertical distances count as horizontal ones.
        let up = voxel_light_weight(Vec3::new(0.0, 0.0, 2.0), BLOCK, 0.0);
        assert!((up - w(2.0, 0.0)).abs() < 1e-7);
        // Blocked by geometry it lights only about 1.2 voxels, a little more
        // at the light itself.
        assert!((w(0.0, 1.0) - 0.25 * 1.25).abs() < 1e-6);
        assert!(w(1.2, 1.0) > 0.0);
        assert_eq!(w(1.25, 1.0), 0.0);
        assert!(w(1.0, 1.0) < w(1.0, 0.0));
    }

    #[test]
    fn voxel_light_box_sums_as_the_original() {
        // At a voxel centre the box gathers 22.0 voxels' worth in open
        // space, and 3.8 where everything around blocks it.
        let at = Vec3::new(100.5, 100.5, 30.5);
        let (lo, hi) = voxel_light_box(at, BLOCK).unwrap();
        assert_eq!((lo, hi), (UVec3::new(98, 98, 28), UVec3::new(104, 104, 35)));
        let sum = |blocked: f32| {
            let mut s = 0.0;
            for z in lo.z..hi.z {
                for y in lo.y..hi.y {
                    for x in lo.x..hi.x {
                        let centre = UVec3::new(x, y, z).as_vec3() + 0.5;
                        s += voxel_light_weight(centre - at, BLOCK, blocked);
                    }
                }
            }
            s / 0.25
        };
        assert!((sum(0.0) - 22.0).abs() < 0.05);
        assert!((sum(1.0) - 3.8).abs() < 0.05);
    }

    #[test]
    fn voxel_light_box_keeps_out_of_margins_and_the_bottom() {
        // Clamped to the 8-column margins, and empty inside them.
        let (lo, hi) = voxel_light_box(Vec3::new(9.2, 247.0, 20.0), BLOCK).unwrap();
        assert_eq!((lo.x, hi.x, lo.y, hi.y), (8, 12, 244, 248));
        assert!(voxel_light_box(Vec3::new(3.0, 100.0, 20.0), BLOCK).is_none());
        // Near the bottom: k + 1 slices from k − 3, none at or below it.
        let (lo, hi) = voxel_light_box(Vec3::new(100.0, 100.0, 4.0), BLOCK).unwrap();
        assert_eq!((lo.z, hi.z), (1, 6));
        let (lo, hi) = voxel_light_box(Vec3::new(100.0, 100.0, 2.0), BLOCK).unwrap();
        assert_eq!((lo.z, hi.z), (0, 2));
        assert!(voxel_light_box(Vec3::new(100.0, 100.0, 0.4), BLOCK).is_none());
        // At the top only the slices inside the block.
        let (lo, hi) = voxel_light_box(Vec3::new(100.0, 100.0, 63.0), BLOCK).unwrap();
        assert_eq!((lo.z, hi.z), (60, 64));
        assert!(voxel_light_box(Vec3::new(f32::NAN, 0.0, 0.0), BLOCK).is_none());
    }

    #[test]
    fn warm_up_progress_counts_the_newest_volume_only() {
        let mut warm = GiWarmUp::default();
        assert_eq!(warm.progress(), 0.0);
        (warm.serial, warm.steps) = (2, 600);
        // A report for the previous volume counts nothing.
        warm.report.store((1 << 32) | 600, Ordering::Release);
        assert_eq!(warm.progress(), 0.0);
        warm.report.store((2 << 32) | 150, Ordering::Release);
        assert_eq!(warm.progress(), 0.25);
        assert!(!warm.finished());
        // Steps past the warm-up (one a frame) keep it finished.
        warm.report.store((2 << 32) | 640, Ordering::Release);
        assert!(warm.finished());
        // Without a warm-up there is nothing to wait for.
        warm.steps = 0;
        assert!(warm.finished());
    }
}
