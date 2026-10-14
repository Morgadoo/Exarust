//! The original's voxel global illumination: a block of 20 cm voxels where
//! light from the level's lights spreads through the open cells, one step
//! per frame, and surfaces take their ambient from it. The occupancy comes
//! from the world (`exa_world::gi_volume`); this module owns the volume
//! textures, the propagation pass (`gi.wesl`) and the lights it injects.
//! Rules: `docs/RENDERING.md` (Voxel GI).

use std::sync::Arc;

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::core_pipeline::Core3dSystems;
use bevy::core_pipeline::schedule::Core3d;
use bevy::prelude::*;
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

use crate::material::{ExaMaterial, flags};

pub struct GiPlugin;

impl Plugin for GiPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "gi.wesl");
        app.init_resource::<GiLights>()
            .add_plugins((
                ExtractResourcePlugin::<GiTextures>::default(),
                ExtractResourcePlugin::<GiLights>::default(),
            ))
            .add_systems(
                PostUpdate,
                (create_textures, gather_lights, give_materials_gi).chain(),
            );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<GiOwed>()
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Core3d, propagate.in_set(Core3dSystems::Prepass));
    }
}

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
    base_ambient: f32,
    /// Steps still owed to the warm-up.
    warm_up: u32,
}

/// A light's share of the GI injection (its record's multiplier); lights
/// without one inject in full.
#[derive(Component, Clone, Copy, Debug)]
pub struct GiMultiplier(pub f32);

/// The lights injected into the volume this frame.
#[derive(Resource, Clone, Default, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiLights(Vec<GiLight>);

#[derive(Clone, Copy, Default, ShaderType)]
struct GiLight {
    position_radius: Vec4,
    colour: Vec4,
    /// Vertical shaping (y, z), as the surface shader reads it.
    shaping: Vec4,
}

const MAX_LIGHTS: usize = 32;

#[derive(Clone, ShaderType)]
struct GiParams {
    origin: Vec4,
    extent: Vec4,
    lights: [GiLight; MAX_LIGHTS],
}

/// Lumens per unit of the original's light intensity (see `exa_world`).
const LUMENS_PER_INTENSITY: f32 = 8.0;
/// Light ranges are 1.25 radii; the volume falls off over one radius.
const REACH: f32 = 1.25;
/// The per-step injection of a light: 0.15 × 0.01 of its colour.
const INJECTION: f32 = 0.0015;

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
    mut images: ResMut<Assets<Image>>,
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
    commands.insert_resource(GiTextures {
        occupancy,
        floors,
        light,
        scratch,
        origin: volume.origin,
        extent,
        base_ambient: volume.base_ambient,
        warm_up: volume.warm_up,
    });
}

fn gather_lights(
    mut gi: ResMut<GiLights>,
    lights: Query<(&PointLight, &GlobalTransform, Option<&GiMultiplier>)>,
    textures: Option<ResMut<GiTextures>>,
) {
    gi.0.clear();
    let Some(mut textures) = textures else {
        return;
    };
    // The warm-up was handed to the render world with the new volume.
    if !textures.is_added() && textures.warm_up > 0 {
        textures.warm_up = 0;
    }
    for (light, at, share) in lights
        .iter()
        .filter(|l| l.2.is_none_or(|m| m.0 > 0.0))
        .take(MAX_LIGHTS)
    {
        let c = light.color.to_linear();
        let intensity = light.intensity / LUMENS_PER_INTENSITY * share.map_or(1.0, |m| m.0);
        gi.0.push(GiLight {
            position_radius: at.translation().extend(light.range / REACH),
            colour: (Vec3::new(c.red, c.green, c.blue) * intensity * INJECTION).extend(0.0),
            shaping: Vec4::new(
                light.shadow_depth_bias,
                light.shadow_normal_bias * crate::NORMAL_BIAS_SCALE,
                0.0,
                0.0,
            ),
        });
    }
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
    let ids: Vec<AssetId<ExaMaterial>> = if textures.is_added() {
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

/// Warm-up steps not yet run (kept until the pipeline is ready).
#[derive(Resource, Default)]
struct GiOwed(u32);

#[allow(clippy::too_many_arguments)]
fn prepare_bind_group(
    mut commands: Commands,
    textures: Option<ResMut<GiTextures>>,
    lights: Res<GiLights>,
    pipeline: Option<Res<GiPipeline>>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut owed: ResMut<GiOwed>,
) {
    let (Some(mut textures), Some(pipeline)) = (textures, pipeline) else {
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
    for (slot, l) in entries.iter_mut().zip(&lights.0) {
        *slot = *l;
    }
    let mut params = UniformBuffer::from(GiParams {
        origin: textures.origin.extend(textures.base_ambient),
        extent: textures
            .extent
            .extend(lights.0.len().min(MAX_LIGHTS) as f32),
        lights: entries,
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
    if textures.warm_up > 0 {
        owed.0 = textures.warm_up;
        textures.warm_up = 0;
    }
    commands.insert_resource(GiBindGroup {
        bind_group,
        _params: params,
    });
}

#[allow(clippy::too_many_arguments)]
fn propagate(
    _view: ViewQuery<&ExtractedView>,
    textures: Option<Res<GiTextures>>,
    bind_group: Option<Res<GiBindGroup>>,
    pipeline: Option<Res<GiPipeline>>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    mut owed: ResMut<GiOwed>,
    mut ctx: RenderContext,
) {
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
    // The warm-up runs at once; then one step per frame.
    let steps = owed.0.max(1);
    owed.0 = 0;
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
}
