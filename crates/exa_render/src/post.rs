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
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId,
    ColorTargetState, ColorWrites, FilterMode, FragmentState, Operations, PipelineCache,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use std::collections::HashMap;

/// The original's sharpen at its 4× supersampling (level 3):
/// 1.5 × (0.075·3 + 0.4).
pub const SHARPEN: f32 = 0.9375;

/// The sharpen the views' final pass takes, from the supersample level the
/// settings hold ([`crate::view::SupersampleLevel::sharpen`]): the level
/// setter stores it on each view, and the post copies it. Every camera with
/// [`ExaPost`] follows it.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct PostSharpen(pub f32);

impl Default for PostSharpen {
    /// The 4× setting's.
    fn default() -> Self {
        Self(SHARPEN)
    }
}

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

    /// The control after the final pass at `clock`: the running fade stops
    /// once it has run out (the screen fade past 5000 ticks; the
    /// desaturation fade, consulted only without a screen fade, past 3000,
    /// clearing its kind with it).
    pub fn expire(self, clock: u32) -> Self {
        let clock = u64::from(clock);
        let started = u64::from(self.start);
        let mut out = self;
        if self.fade {
            if started + u64::from(FADE_TICKS) < clock {
                out.fade = false;
            }
        } else if self.desaturate && started + u64::from(DESAT_FADE_TICKS) < clock {
            out.desaturate = false;
            out.desaturate_kind = 0;
        }
        out
    }
}

/// A fade the game asks for; the frame stamps it with its clock. The
/// simulation step's callers (the teleport bubble) send it as a message.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FadeRequest {
    /// The screen fade in from black (the story's start and the revive).
    Screen,
    /// The desaturation fade with its kind byte (the mind's awakening with
    /// 0, the teleport bubble's hold with 1).
    Desaturate(u8),
}

/// The main view's post processor's fade state: the control the final
/// pass reads and the starts asked for since the last frame.
#[derive(Resource, Clone, Debug)]
pub struct ScreenFade {
    pub control: FadeControl,
    pub requests: Vec<FadeRequest>,
}

impl Default for ScreenFade {
    fn default() -> Self {
        Self {
            control: FadeControl {
                start: 0,
                fade: false,
                desaturate: false,
                desaturate_kind: 0,
            },
            requests: Vec::new(),
        }
    }
}

impl ScreenFade {
    /// Starts the screen fade (the post processor's fade start).
    pub fn begin_fade(&mut self) {
        self.requests.push(FadeRequest::Screen);
    }

    /// Starts the desaturation fade with `kind` (the post processor's
    /// desaturation start).
    pub fn begin_desaturate(&mut self, kind: u8) {
        self.requests.push(FadeRequest::Desaturate(kind));
    }

    /// One frame at `clock`: the requests start their fades, then the
    /// values the final pass takes are worked out and a fade that has run
    /// out stops.
    pub fn step(&mut self, clock: u32, base_bloom: f32) -> PostFade {
        for request in self.requests.drain(..) {
            self.control = match request {
                FadeRequest::Screen => FadeControl::begin_fade(clock, self.control),
                FadeRequest::Desaturate(kind) => {
                    FadeControl::begin_desaturate(clock, kind, self.control)
                }
            };
        }
        let values = fade_values(clock, self.control, base_bloom);
        self.control = self.control.expire(clock);
        values
    }
}

/// The four components the final pass's `Blur` uniform receives each frame,
/// in order: the screen fade's weight, the bloom's scale (the post
/// processor's bloom strength, faded in with the screen), the desaturation
/// mask (how far the scene is replaced by its bloom) and the bloom term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostFade {
    pub weight: f32,
    pub bloom_scale: f32,
    pub mask: f32,
    pub bloom: f32,
}

/// The fade values at `clock` for a control and the base bloom strength.
/// With no fade active the weight is 1 and the base strength passes through.
///
/// The screen fade's alpha is `1 - (start + 5000 - clock) * 0.0002` computed
/// in double precision (0 at the start, 1 at the end, 1 once it has run out,
/// when the fade also stops); the weight is the alpha to the tenth power
/// (one square, then four more factors of it in f32) and the bloom scale
/// is `alpha · base`.
///
/// The desaturation fade's ramp is `1 - (start + 3000 - clock) / 3000` (1
/// once it has run out); below a quarter its mask is `1 - 4·ramp`, else
/// `(ramp - 0.25) · 1.333` with the product in double precision. The mask
/// the pass receives is `1 - mask`, the weight stays 1 and the base
/// strength passes through; a non-zero kind adds `(1 - mask)² · 0.4`
/// (a double product) to the bloom. The frame in which it runs out still
/// computes the mask from a ramp of 1 and has no bloom.
pub fn fade_values(clock: u32, control: FadeControl, base_bloom: f32) -> PostFade {
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
            bloom_scale: alpha * base_bloom,
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
            bloom_scale: base_bloom,
            mask: 1.0 - mask,
            bloom,
        };
    }
    PostFade {
        weight: 1.0,
        bloom_scale: base_bloom,
        mask: 0.0,
        bloom: 0.0,
    }
}

/// One bloom blur kernel as the filter uploads it: the first `taps`
/// weights and `taps − 1`, the loop's last index.
#[derive(Debug, Clone, PartialEq)]
pub struct BloomKernel {
    pub weights: Vec<f32>,
    pub last: i32,
}

/// The bloom filter's weights for `n` samples: `x = (3n − i)/3n − 0.4375`
/// (the `3n` an integer product converted once), `w = x²·24`, summed with
/// the first weight once and every other twice. The legacy workflow scales
/// each normalised weight by 1.5 in single precision; the linear one by
/// `level·0.016 + 1`, the scale and the product in double.
pub fn bloom_weights(n: usize, level: f32, linear: bool) -> Vec<f32> {
    let three = (n as i64 * 3) as f32;
    let mut weights = Vec::with_capacity(n);
    let mut sum = 0.0f32;
    for i in 0..n {
        let x = (three - i as f32) / three - 0.4375;
        let w = x * x * 24.0;
        weights.push(w);
        sum += if i < 1 { w } else { w * 2.0 };
    }
    for w in &mut weights {
        *w = if linear {
            ((f64::from(level) * 0.016 + 1.0) * f64::from(*w / sum)) as f32
        } else {
            *w / sum * 1.5
        };
    }
    weights
}

/// The bloom kernel for a tap count the original rounds from a float with
/// the processor's 64-bit conversion (nearest, ties to even; an
/// out-of-range value gives the integer indefinite, whose low 32 bits are
/// zero). The filter computes one weight more than it uploads: the curve
/// spans `taps + 1` samples and the first `taps` are kept.
pub fn bloom_kernel(taps: f32, level: f32, linear: bool) -> BloomKernel {
    let taps = crate::sun_fit::convert_nearest(f64::from(taps)) as i32;
    let computed = usize::try_from(taps.wrapping_add(1)).unwrap_or(0);
    let mut weights = bloom_weights(computed, level, linear);
    weights.truncate(usize::try_from(taps).unwrap_or(0));
    BloomKernel {
        weights,
        last: taps.wrapping_sub(1),
    }
}

/// Which of the two cached post programs the pass runs: the bright variant
/// (the sharpen and desaturation uniforms active) when the sharpen amount,
/// widened to double, is above 0.01 (a NaN is not) and the shader-restore
/// switch is clear, else the plain one. Each variant keeps its own program
/// and uniform locations; a missing program is fetched once and
/// remembered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostVariant {
    Plain,
    Bright,
}

/// The variant for a sharpen amount and the renderer's shader-restore
/// switch.
pub fn post_variant(sharpness: f32, restore_shaders: bool) -> PostVariant {
    if f64::from(sharpness) > 0.01 && !restore_shaders {
        PostVariant::Bright
    } else {
        PostVariant::Plain
    }
}

/// The two variants' cached program ids and uniform locations; `None`
/// until the variant is first fetched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PostPrograms {
    plain: Option<u32>,
    bright: Option<u32>,
}

impl PostPrograms {
    /// The program for a variant, fetched through `fetch` and remembered.
    pub fn get(&mut self, variant: PostVariant, fetch: impl FnOnce() -> u32) -> u32 {
        let slot = match variant {
            PostVariant::Plain => &mut self.plain,
            PostVariant::Bright => &mut self.bright,
        };
        *slot.get_or_insert_with(fetch)
    }

    /// The cached program of a variant.
    pub fn cached(&self, variant: PostVariant) -> Option<u32> {
        match variant {
            PostVariant::Plain => self.plain,
            PostVariant::Bright => self.bright,
        }
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
    /// The final pass's `Blur`: the fade's weight, the bloom's scale, the
    /// desaturation mask and the bloom term ([`PostFade`]), written each
    /// frame from [`ScreenFade`].
    pub blur: Vec4,
}

impl Default for ExaPost {
    fn default() -> Self {
        Self {
            bloom_strength: 1.0,
            desaturate: 0.0,
            sharpen: SHARPEN,
            pad1: 0.0,
            blur: Vec4::new(1.0, 1.0, 0.0, 0.0),
        }
    }
}

/// The frame's fade: each post camera's `Blur` from [`ScreenFade`] at the
/// frame clock (game time in whole milliseconds, paused time held, as the
/// original's frame clock) and the camera's bloom strength.
fn step_screen_fade(
    time: Res<Time>,
    mut fade: ResMut<ScreenFade>,
    mut sent: MessageReader<FadeRequest>,
    mut posts: Query<&mut ExaPost>,
) {
    fade.requests.extend(sent.read().copied());
    if !fade.requests.is_empty() {
        info!("screen fade: {:?} at {:?}", fade.requests, time.elapsed());
    }
    let clock = time.elapsed().as_millis() as u32;
    let busy = fade.control.fade || fade.control.desaturate || !fade.requests.is_empty();
    if !busy {
        for mut post in &mut posts {
            let idle = Vec4::new(1.0, post.bloom_strength, 0.0, 0.0);
            if post.blur != idle {
                post.blur = idle;
            }
        }
        return;
    }
    // The scale is the fade's alpha times each camera's own strength.
    let v = fade.step(clock, 1.0);
    for mut post in &mut posts {
        let scale = v.bloom_scale * post.bloom_strength;
        post.blur = Vec4::new(v.weight, scale, v.mask, v.bloom);
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
        .init_resource::<PostSharpen>()
        .init_resource::<ScreenFade>()
        .add_message::<FadeRequest>()
        .add_systems(
            PostUpdate,
            (adopt_cameras, follow_sharpen, step_screen_fade)
                .chain()
                .in_set(exa_physics::FramePostPhase::Prepare),
        );
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

/// A 3D camera just spawned that draws to the window or a plain image: not
/// a supersampled scene's and not an off-screen scene's.
type NewCamera = (
    Added<Camera3d>,
    Without<crate::supersample::Supersample>,
    Without<crate::supersample::Offscreen>,
);

/// Gives every new 3D camera the original's post-processing instead of
/// Bevy's tonemapping.
fn adopt_cameras(
    mut commands: Commands,
    cameras: Query<Entity, NewCamera>,
    sharpen: Option<Res<PostSharpen>>,
) {
    let sharpen = sharpen.map_or(SHARPEN, |s| s.0);
    for camera in &cameras {
        commands.entity(camera).insert((
            Hdr,
            Tonemapping::None,
            ExaPost {
                sharpen,
                ..ExaPost::default()
            },
        ));
    }
}

/// Gives every post camera the settings' sharpen (a camera given its post
/// elsewhere, as the supersampled display camera is, takes it too).
fn follow_sharpen(sharpen: Res<PostSharpen>, mut posts: Query<&mut ExaPost>) {
    for mut post in &mut posts {
        if post.sharpen != sharpen.0 {
            post.sharpen = sharpen.0;
        }
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
        let mut target = |label: &'static str, divisor: u32, format, usage| {
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
                    usage: TextureUsages::RENDER_ATTACHMENT
                        | TextureUsages::TEXTURE_BINDING
                        | usage,
                    view_formats: &[],
                },
            )
        };
        let none = TextureUsages::empty();
        // The bright pass is also the scene's high field the next frame's
        // sharp refractions read (copied out after the pass).
        let bright = target("exa_post_bright", 2, BRIGHT_FORMAT, TextureUsages::COPY_SRC);
        let blurred_x = target("exa_post_blur_x", 4, BRIGHT_FORMAT, none);
        // The window's blurred texture is also the scene's low field the
        // next frame's local reflections read (copied out after the pass).
        let blurred = target("exa_post_blur", 4, BLURRED_FORMAT, TextureUsages::COPY_SRC);
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
        &ExtractedCamera,
        &PostTextures,
        &DynamicUniformIndex<ExaPost>,
    )>,
    low_field: Option<Res<crate::local_reflection::SceneLowField>>,
    high_field: Option<Res<crate::refraction::SceneHighField>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut mismatch_logged: Local<bool>,
    pipelines: Option<Res<PostPipelines>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    uniforms: Res<ComponentUniforms<ExaPost>>,
    mut ctx: RenderContext,
) {
    let (target, extracted, camera, textures, index) = view.into_inner();
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
    // The post chain's last camera (the window's, or the off-screen
    // frame's) hands its blurred bright pass to the next frame's local
    // reflections and refractions (the low field), and its bright pass to
    // the next frame's sharp refractions (the high field); an image target
    // of the post chain is that frame.
    if camera.target.is_none() {
        return;
    }
    let fields = [
        (low_field.map(|f| f.image.clone()), &textures.blurred, "low"),
        (
            high_field.map(|f| f.image.clone()),
            &textures.bright,
            "high",
        ),
    ];
    for (field, source, name) in fields {
        let Some(image) = field.as_ref().and_then(|f| gpu_images.get(f)) else {
            continue;
        };
        if image.texture.size() != source.texture.size() {
            if !*mismatch_logged {
                *mismatch_logged = true;
                debug!(
                    "scene field: the {name} field {:?} and its post pass {:?} differ; not copied",
                    image.texture.size(),
                    source.texture.size()
                );
            }
            continue;
        }
        ctx.command_encoder().copy_texture_to_texture(
            source.texture.as_image_copy(),
            image.texture.as_image_copy(),
            image.texture.size(),
        );
    }
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
    fn without_a_fade_the_base_bloom_passes_through() {
        let values = fade_values(1234, idle(), 0.6);
        assert_eq!(
            values,
            PostFade {
                weight: 1.0,
                bloom_scale: 0.6,
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
        assert_eq!((start.weight, start.bloom_scale), (0.0, 0.0));
        // Halfway: alpha 0.5, desaturation 0.4, weight 0.5^10.
        let half = fade_values(3500, fade, 0.8);
        assert!((half.weight - 0.000_976_562_5).abs() < 1e-7);
        assert!((half.bloom_scale - 0.4).abs() < 1e-6);
        // At the end the alpha reaches 1 and the base desaturation returns.
        let end = fade_values(6000, fade, 0.8);
        assert!((end.weight - 1.0).abs() < 1e-6);
        assert!((end.bloom_scale - 0.8).abs() < 1e-6);
        // Past it the alpha stays 1: the same values, mask and bloom zero.
        let done = fade_values(6001, fade, 0.8);
        assert_eq!(
            done,
            PostFade {
                weight: 1.0,
                bloom_scale: 0.8,
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
            (start.mask, start.weight, start.bloom_scale),
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
            (done.bloom, done.bloom_scale, done.weight),
            (0.0, 0.75, 1.0)
        );
    }
    #[test]
    fn a_run_out_fade_stops_and_the_desaturation_clears_its_kind() {
        let fade = FadeControl::begin_fade(1000, idle());
        assert!(fade.expire(6000).fade);
        assert!(!fade.expire(6001).fade);
        let desat = FadeControl::begin_desaturate(0, 1, idle());
        let done = desat.expire(3001);
        assert!(!done.desaturate && done.desaturate_kind == 0);
        // Under a screen fade the desaturation waits.
        let both = FadeControl::begin_fade(0, desat);
        assert!(both.expire(3001).desaturate);
    }

    /// The live frame system: a requested fade starts at the frame clock,
    /// writes every post camera's `Blur` (black at its start) and, once run
    /// out, leaves the idle values.
    #[test]
    fn the_frame_fades_every_post_camera_from_black() {
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .init_resource::<ScreenFade>()
            .add_message::<FadeRequest>()
            .add_systems(Update, step_screen_fade);
        let camera = app.world_mut().spawn(ExaPost::default()).id();
        app.update();
        let blur = |app: &App| app.world().get::<ExaPost>(camera).unwrap().blur;
        assert_eq!(blur(&app), Vec4::new(1.0, 1.0, 0.0, 0.0));
        app.world_mut().resource_mut::<ScreenFade>().begin_fade();
        app.update();
        let start = blur(&app);
        assert_eq!((start.x, start.y), (0.0, 0.0));
        for _ in 0..25 {
            app.update();
        }
        let half = blur(&app);
        assert!(half.x > 0.0 && half.x < 0.01 && half.y > 0.4 && half.y < 0.6);
        for _ in 0..30 {
            app.update();
        }
        assert!(!app.world().resource::<ScreenFade>().control.fade);
        app.update();
        assert_eq!(blur(&app), Vec4::new(1.0, 1.0, 0.0, 0.0));
    }

    #[test]
    fn the_post_params_pick_the_bright_variant() {
        assert_eq!(post_variant(0.9375, false), PostVariant::Bright);
        assert_eq!(post_variant(0.01, false), PostVariant::Plain);
        assert_eq!(post_variant(0.0, false), PostVariant::Plain);
        assert_eq!(post_variant(0.9375, true), PostVariant::Plain);
        assert_eq!(post_variant(f32::NAN, false), PostVariant::Plain);
        let mut programs = PostPrograms::default();
        let mut fetches = 0;
        assert_eq!(
            programs.get(PostVariant::Bright, || {
                fetches += 1;
                7
            }),
            7
        );
        assert_eq!(
            programs.get(PostVariant::Bright, || {
                fetches += 1;
                9
            }),
            7
        );
        assert_eq!(fetches, 1);
        assert_eq!(programs.cached(PostVariant::Plain), None);
    }
    #[test]
    fn the_bloom_kernel_weights_follow_the_original_curve() {
        // Five samples: x = (15 − i)/15 − 0.4375, w = x²·24, doubled except
        // the first, normalised and scaled by 1.5.
        let weights = bloom_weights(5, 0.0, false);
        let raw: Vec<f32> = (0..5)
            .map(|i| {
                let x = (15.0 - i as f32) / 15.0 - 0.4375;
                x * x * 24.0
            })
            .collect();
        let mut sum = raw[0];
        for r in &raw[1..] {
            sum += r * 2.0;
        }
        for (w, r) in weights.iter().zip(&raw) {
            assert_eq!(*w, r / sum * 1.5);
        }
        // The linear workflow scales by level·0.016 + 1 in double.
        let linear = bloom_weights(5, 4.0, true);
        assert_eq!(
            linear[0],
            ((4.0f64 * 0.016 + 1.0) * f64::from(raw[0] / sum)) as f32
        );
        // Five taps: the curve spans six samples and five are uploaded.
        let kernel = bloom_kernel(5.4, 0.0, false);
        assert_eq!(kernel.last, 4);
        assert_eq!(kernel.weights, bloom_weights(6, 0.0, false)[..5]);
        assert_eq!(bloom_kernel(5.5, 0.0, false).last, 5);
        assert_eq!(bloom_kernel(6.5, 0.0, false).last, 5);
        assert!(bloom_kernel(0.0, 0.0, false).weights.is_empty());
        // An indefinite count keeps its low 32 bits: no taps, last −1.
        let kernel = bloom_kernel(f32::NAN, 0.0, false);
        assert!(kernel.weights.is_empty());
        assert_eq!(kernel.last, -1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every post camera takes the settings' sharpen, the ones adopted
    /// before a change too.
    #[test]
    fn the_post_cameras_follow_the_sharpen() {
        let mut app = App::new();
        app.init_resource::<PostSharpen>()
            .add_systems(Update, follow_sharpen);
        let camera = app.world_mut().spawn(ExaPost::default()).id();
        app.update();
        assert_eq!(app.world().get::<ExaPost>(camera).unwrap().sharpen, SHARPEN);
        app.world_mut().resource_mut::<PostSharpen>().0 = 0.0;
        app.update();
        assert_eq!(app.world().get::<ExaPost>(camera).unwrap().sharpen, 0.0);
    }
}
