//! The original's post-processing chain, run on every 3D camera in place of
//! Bevy's tonemapping: the lit scene clamps at 1 (the original stores it in
//! 8-bit sRGB), a bright pass at half resolution feeds a 31-tap separable
//! blur at a quarter, and the final pass applies the always-on grading curve
//! and combines the bloom like a screen. The output is encoded to sRGB by
//! the window. Rules: `docs/RENDERING.md` (Post-processing).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::camera::Hdr;
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::tonemapping::{Tonemapping, tonemapping};
use bevy::core_pipeline::{Core3dSystems, FullscreenShader};
use bevy::image::ToExtents;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_component::{
    ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
    UniformComponentPlugin,
};
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId,
    ColorTargetState, ColorWrites, FilterMode, FragmentState, Operations, PipelineCache,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use std::collections::HashMap;

/// The original's post-processing settings, on a 3D camera.
#[derive(Component, ExtractComponent, ShaderType, Clone, Copy, Debug)]
#[extract_app(RenderApp)]
pub struct ExaPost {
    /// Bloom strength: 1 in play.
    pub bloom_strength: f32,
    /// Desaturation: 0 in play, faded in for transitions.
    pub desaturate: f32,
    pad0: f32,
    pad1: f32,
}

impl Default for ExaPost {
    fn default() -> Self {
        Self {
            bloom_strength: 1.0,
            desaturate: 0.0,
            pad0: 0.0,
            pad1: 0.0,
        }
    }
}

pub struct PostPlugin;

impl Plugin for PostPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "post.wesl");
        app.add_plugins((
            ExtractComponentPlugin::<ExaPost>::default(),
            UniformComponentPlugin::<ExaPost>::default(),
        ))
        .add_systems(PostUpdate, adopt_cameras);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(
                Render,
                (
                    prepare_textures.in_set(RenderSystems::PrepareResources),
                    prepare_finish.in_set(RenderSystems::Prepare),
                ),
            )
            .add_systems(
                Core3d,
                post_process
                    .in_set(Core3dSystems::PostProcess)
                    .before(tonemapping),
            );
    }
}

/// Gives every new 3D camera the original's post-processing instead of
/// Bevy's tonemapping.
fn adopt_cameras(mut commands: Commands, cameras: Query<Entity, Added<Camera3d>>) {
    for camera in &cameras {
        commands
            .entity(camera)
            .insert((Hdr, Tonemapping::None, ExaPost::default()));
    }
}

#[derive(Resource)]
struct PostPipelines {
    single: BindGroupLayoutDescriptor,
    combine: BindGroupLayoutDescriptor,
    sampler: Sampler,
    bright: CachedRenderPipelineId,
    blur_x: CachedRenderPipelineId,
    blur_y: CachedRenderPipelineId,
    /// Final pass, by view target format.
    finish: HashMap<TextureFormat, CachedRenderPipelineId>,
    shader: Handle<Shader>,
    vertex: bevy::render::render_resource::VertexState,
}

fn pipeline_descriptor(
    entry: &'static str,
    layout: &BindGroupLayoutDescriptor,
    format: TextureFormat,
    shader: &Handle<Shader>,
    vertex: &bevy::render::render_resource::VertexState,
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: Some(format!("exa_post_{entry}").into()),
        layout: vec![layout.clone()],
        vertex: vertex.clone(),
        fragment: Some(FragmentState {
            shader: shader.clone(),
            entry_point: Some(entry.into()),
            targets: vec![Some(ColorTargetState {
                format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    }
}

/// Bright pass and blur targets: 8-bit linear like the original, the last
/// blur stored as sRGB.
const BRIGHT_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const BLURRED_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

fn init_pipelines(
    mut commands: Commands,
    device: Res<RenderDevice>,
    assets: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
) {
    let single = BindGroupLayoutDescriptor::new(
        "exa_post_single",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    let combine = BindGroupLayoutDescriptor::new(
        "exa_post_combine",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                uniform_buffer::<ExaPost>(true),
            ),
        ),
    );
    let shader =
        assets.load(AssetPath::from_path_buf(embedded_path!("post.wesl")).with_source("embedded"));
    let vertex = fullscreen.to_vertex_state();
    let pipeline = |entry: &'static str, layout: &BindGroupLayoutDescriptor, format| {
        cache.queue_render_pipeline(pipeline_descriptor(entry, layout, format, &shader, &vertex))
    };
    let pipelines = PostPipelines {
        bright: pipeline("bright", &single, BRIGHT_FORMAT),
        blur_x: pipeline("blur_x", &single, BRIGHT_FORMAT),
        blur_y: pipeline("blur_y", &single, BLURRED_FORMAT),
        finish: HashMap::new(),
        shader: shader.clone(),
        vertex: vertex.clone(),
        sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        single,
        combine,
    };
    commands.insert_resource(pipelines);
}

/// Queues the final pass for each view target format in use.
fn prepare_finish(
    mut pipelines: ResMut<PostPipelines>,
    cache: Res<PipelineCache>,
    views: Query<&ExtractedView, With<ExaPost>>,
) {
    for view in &views {
        let format = view.target_format;
        if !pipelines.finish.contains_key(&format) {
            let id = cache.queue_render_pipeline(pipeline_descriptor(
                "finish",
                &pipelines.combine,
                format,
                &pipelines.shader,
                &pipelines.vertex,
            ));
            pipelines.finish.insert(format, id);
        }
    }
}

/// The bloom targets of a view.
#[derive(Component)]
struct PostTextures {
    bright: CachedTexture,
    blurred_x: CachedTexture,
    blurred: CachedTexture,
}

fn prepare_textures(
    mut commands: Commands,
    mut textures: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<ExaPost>>,
) {
    for (entity, camera) in &views {
        let Some(size) = camera.physical_target_size else {
            continue;
        };
        let mut target = |label: &'static str, divisor: u32, format| {
            let mut extent = size.to_extents();
            extent.width = (extent.width / divisor).max(1);
            extent.height = (extent.height / divisor).max(1);
            textures.get(
                &device,
                TextureDescriptor {
                    label: Some(label),
                    size: extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        let bright = target("exa_post_bright", 2, BRIGHT_FORMAT);
        let blurred_x = target("exa_post_blur_x", 4, BRIGHT_FORMAT);
        let blurred = target("exa_post_blur", 4, BLURRED_FORMAT);
        commands.entity(entity).insert(PostTextures {
            bright,
            blurred_x,
            blurred,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn post_process(
    view: ViewQuery<(
        &ViewTarget,
        &ExtractedView,
        &PostTextures,
        &DynamicUniformIndex<ExaPost>,
    )>,
    pipelines: Option<Res<PostPipelines>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    uniforms: Res<ComponentUniforms<ExaPost>>,
    mut ctx: RenderContext,
) {
    let (target, extracted, textures, index) = view.into_inner();
    let Some(pipelines) = pipelines else { return };
    let Some(&finish_id) = pipelines.finish.get(&extracted.target_format) else {
        return;
    };
    let (Some(bright), Some(blur_x), Some(blur_y), Some(finish), Some(settings)) = (
        cache.get_render_pipeline(pipelines.bright),
        cache.get_render_pipeline(pipelines.blur_x),
        cache.get_render_pipeline(pipelines.blur_y),
        cache.get_render_pipeline(finish_id),
        uniforms.uniforms().binding(),
    ) else {
        return;
    };
    let single_layout = cache.get_bind_group_layout(&pipelines.single);
    let single = |source: &TextureView| {
        device.create_bind_group(
            "exa_post_single",
            &single_layout,
            &BindGroupEntries::sequential((source, &pipelines.sampler)),
        )
    };
    let mut pass = |label: &str,
                    pipeline,
                    bind_group: &bevy::render::render_resource::BindGroup,
                    dynamic: &[u32],
                    output: &TextureView| {
        let mut render_pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, dynamic);
        render_pass.draw(0..3, 0..1);
    };
    let main = single(target.main_texture_view());
    pass(
        "exa_post_bright",
        bright,
        &main,
        &[],
        &textures.bright.default_view,
    );
    let from_bright = single(&textures.bright.default_view);
    pass(
        "exa_post_blur_x",
        blur_x,
        &from_bright,
        &[],
        &textures.blurred_x.default_view,
    );
    let from_x = single(&textures.blurred_x.default_view);
    pass(
        "exa_post_blur_y",
        blur_y,
        &from_x,
        &[],
        &textures.blurred.default_view,
    );
    let post = target.post_process_write();
    let combine = device.create_bind_group(
        "exa_post_combine",
        &cache.get_bind_group_layout(&pipelines.combine),
        &BindGroupEntries::sequential((
            post.source,
            &pipelines.sampler,
            &textures.blurred.default_view,
            settings,
        )),
    );
    pass(
        "exa_post_finish",
        finish,
        &combine,
        &[index.index()],
        post.destination,
    );
}
