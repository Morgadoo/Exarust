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

/// The original's sharpen at its 4× supersampling (level 3):
/// 1.5 × (0.075·3 + 0.4).
pub const SHARPEN: f32 = 0.9375;

/// The screen fade lasts this many clock ticks (5 s at the engine's 1 ms
/// clock); its alpha rises from 0 to 1 by 1/5000 a tick.
pub const FADE_TICKS: u32 = 5000;
/// A desaturation fade lasts this many ticks (3 s); its ramp rises by
/// 1/3000 a tick.
pub const DESAT_FADE_TICKS: u32 = 3000;
/// The stored per-tick steps, as doubles: the original multiplies the ticks
/// left by these rather than dividing.
const FADE_STEP: f64 = 0.000_2;
const DESAT_FADE_STEP: f64 = 0.000_333_333_333_333_333_3;

/// The two fade controls the game main and the selection UI start. A fade
/// stores the clock it started at and which of the two runs; the
/// desaturation fade also has a kind byte that adds a bloom term. The two
/// fades share the start clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FadeControl {
    pub start: u32,
    pub fade: bool,
    pub desaturate: bool,
    pub desaturate_kind: u8,
}

impl FadeControl {
    /// Starts the screen fade at `clock`: the other fade's flags stay as
    /// they were.
    pub fn begin_fade(clock: u32, previous: Self) -> Self {
        Self {
            start: clock,
            fade: true,
            ..previous
        }
    }

    /// Starts a desaturation fade at `clock` with a kind.
    pub fn begin_desaturate(clock: u32, kind: u8, previous: Self) -> Self {
        Self {
            start: clock,
            desaturate: true,
            desaturate_kind: kind,
            ..previous
        }
    }
}

/// The four components the final pass's `Blur` uniform receives each frame,
/// in order: the screen fade's weight, the desaturation, the desaturation
/// mask and the bloom term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostFade {
    pub weight: f32,
    pub desaturation: f32,
    pub mask: f32,
    pub bloom: f32,
}

/// The fade values at `clock` for a control and the base desaturation. With
/// no fade active the weight is 1 and the base desaturation passes through.
///
/// The screen fade's alpha is `1 - (start + 5000 - clock) * 0.0002` computed
/// in double precision (0 at the start, 1 at the end, 1 once it has run out,
/// when the fade also stops); the weight is the alpha to the tenth power
/// (one square, then four more factors of it in f32) and the desaturation
/// is `alpha · base`.
///
/// The desaturation fade's ramp is `1 - (start + 3000 - clock) / 3000` (1
/// once it has run out); below a quarter its mask is `1 - 4·ramp`, else
/// `(ramp - 0.25) · 1.333` with the product in double precision. The mask
/// the pass receives is `1 - mask`, the weight stays 1 and the base
/// desaturation passes through; a non-zero kind adds `(1 - mask)² · 0.4`
/// (a double product) to the bloom. The frame in which it runs out still
/// computes the mask from a ramp of 1 and has no bloom.
pub fn fade_values(clock: u32, control: FadeControl, base_desaturation: f32) -> PostFade {
    let clock = u64::from(clock);
    let started = u64::from(control.start);
    if control.fade {
        let until = started + u64::from(FADE_TICKS);
        let alpha = if until < clock {
            1.0f32
        } else {
            (1.0 - (until - clock) as f64 * FADE_STEP) as f32
        };
        let squared = alpha * alpha;
        return PostFade {
            weight: squared * squared * squared * squared * squared,
            desaturation: alpha * base_desaturation,
            mask: 0.0,
            bloom: 0.0,
        };
    }
    if control.desaturate {
        let until = started + u64::from(DESAT_FADE_TICKS);
        let expired = until < clock;
        let ramp = if expired {
            1.0f32
        } else {
            (1.0 - (until - clock) as f64 * DESAT_FADE_STEP) as f32
        };
        let mask = if ramp < 0.25 {
            1.0 - ramp * 4.0
        } else {
            (f64::from(ramp - 0.25) * 1.333) as f32
        };
        let bloom = if control.desaturate_kind != 0 && !expired {
            let rest = 1.0 - mask;
            (f64::from(rest * rest) * 0.4) as f32
        } else {
            0.0
        };
        return PostFade {
            weight: 1.0,
            desaturation: base_desaturation,
            mask: 1.0 - mask,
            bloom,
        };
    }
    PostFade {
        weight: 1.0,
        desaturation: base_desaturation,
        mask: 0.0,
        bloom: 0.0,
    }
}

/// The original's post-processing settings, on a 3D camera.
#[derive(Component, ExtractComponent, ShaderType, Clone, Copy, Debug)]
#[extract_app(RenderApp)]
pub struct ExaPost {
    /// Bloom strength: 1 in play.
    pub bloom_strength: f32,
    /// Desaturation: 0 in play, faded in for transitions.
    pub desaturate: f32,
    /// The sharpen supersampling turns on: 1.5 × (0.075·n + 0.4) at
    /// supersample level n (0 without supersampling).
    pub sharpen: f32,
    pad1: f32,
}

impl Default for ExaPost {
    fn default() -> Self {
        Self {
            bloom_strength: 1.0,
            desaturate: 0.0,
            sharpen: SHARPEN,
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
fn adopt_cameras(
    mut commands: Commands,
    cameras: Query<Entity, (Added<Camera3d>, Without<crate::supersample::Supersample>)>,
) {
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

#[cfg(test)]
mod fade_tests {
    use super::*;

    fn idle() -> FadeControl {
        FadeControl {
            start: 0,
            fade: false,
            desaturate: false,
            desaturate_kind: 0,
        }
    }

    #[test]
    fn without_a_fade_the_base_desaturation_passes_through() {
        let values = fade_values(1234, idle(), 0.6);
        assert_eq!(
            values,
            PostFade {
                weight: 1.0,
                desaturation: 0.6,
                mask: 0.0,
                bloom: 0.0
            }
        );
    }

    #[test]
    fn the_fade_rises_over_five_seconds_and_weighs_the_tenth_power() {
        let fade = FadeControl::begin_fade(1000, idle());
        assert!(fade.fade && !fade.desaturate);
        // The ramp starts at alpha 0 and rises.
        let start = fade_values(1000, fade, 0.8);
        assert_eq!((start.weight, start.desaturation), (0.0, 0.0));
        // Halfway: alpha 0.5, desaturation 0.4, weight 0.5^10.
        let half = fade_values(3500, fade, 0.8);
        assert!((half.weight - 0.000_976_562_5).abs() < 1e-7);
        assert!((half.desaturation - 0.4).abs() < 1e-6);
        // At the end the alpha reaches 1 and the base desaturation returns.
        let end = fade_values(6000, fade, 0.8);
        assert!((end.weight - 1.0).abs() < 1e-6);
        assert!((end.desaturation - 0.8).abs() < 1e-6);
        // Past it the alpha stays 1: the same values, mask and bloom zero.
        let done = fade_values(6001, fade, 0.8);
        assert_eq!(
            done,
            PostFade {
                weight: 1.0,
                desaturation: 0.8,
                mask: 0.0,
                bloom: 0.0
            }
        );
    }

    #[test]
    fn the_desaturation_fade_ramps_mask_and_kind_bloom() {
        let fade = FadeControl::begin_desaturate(500, 0, idle());
        // At the start the ramp is 0, so the mask is 1 and then inverted.
        let start = fade_values(500, fade, 0.75);
        assert_eq!(
            (start.mask, start.weight, start.desaturation),
            (0.0, 1.0, 0.75)
        );
        // Halfway (ramp 0.5): (0.5 - 0.25) * 1.333 = 0.33325, inverted.
        let half = fade_values(2000, fade, 0.75);
        assert!((half.mask - (1.0 - 0.333_25)).abs() < 1e-6);
        assert_eq!(half.bloom, 0.0);
        // The kind adds (1 - mask)^2 * 0.4 at the same point.
        let kind = FadeControl::begin_desaturate(500, 1, idle());
        let with_kind = fade_values(2000, kind, 0.75);
        let expected = (1.0 - 0.333_25f32) * (1.0 - 0.333_25) * 0.4;
        assert!((with_kind.bloom - expected).abs() < 1e-6);
        // The low ramp branch: at ramp 1/6 the mask is 1 - 4 * ramp, then
        // inverted, so 2/3.
        let low = fade_values(1000, fade, 0.75);
        assert!((low.mask - 2.0 / 3.0).abs() < 1e-6);
        // Past the end the ramp is 1, the mask 0.75 * 1.333 = 0.99975 and
        // the pass receives its inverse; the kind's bloom is gone.
        let done = fade_values(3501, kind, 0.75);
        assert!((done.mask - 0.000_25).abs() < 1e-6);
        assert_eq!(
            (done.bloom, done.desaturation, done.weight),
            (0.0, 0.75, 1.0)
        );
    }
}
