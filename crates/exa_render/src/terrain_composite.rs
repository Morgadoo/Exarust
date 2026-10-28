//! Where the terrain's offscreen composite sets live in the renderer, and
//! the pass that bakes a block into them.
//!
//! The original bakes each selected 6 m block into a colour, tangent-space
//! normal, gloss and half-size parallax-height texture, at 1024² for the 21
//! high-resolution sets and 512² for the 43 low ones, and draws those
//! textures instead of stacking layers. This module hosts the sets as
//! render-target images of the original's formats and sizes and runs the
//! bake: [`TerrainBakeQueue`] takes one block's layers and a set, and the
//! render world draws the compositor's four passes into the set (normal,
//! colour, gloss, then the half-size displacement), each a full-target quad
//! over the block with the per-pixel draw's layer stacking
//! (`terrain_bake.wesl`). The live selector hands its set assignments over
//! through [`TerrainCompositeHost::sync`].
//!
//! A set's images are created the first time the selector holds it and
//! dropped when the set goes back to its pool. The full pools are about
//! 235 MB of render targets, and drawing from the sets changes the terrain's
//! look (8-bit composites at about 170 texels a metre instead of the layers
//! read per pixel), so the allocation, the bake and the drawing are opt-in
//! with `EXA_TERRAIN_SETS=1`; the selector's rules run either way.
//!
//! Evidence: the composite set allocator, the compositor driver and the set
//! copies (see `exa_world::terrain::composite`), and the shipped
//! `shadetrn` programs for how the draw reads a set. The texture formats are
//! the OpenGL internal formats the original's sets are created with: RGB8
//! colour, RG8 normal, R8 gloss and R8 half-size displacement. wgpu cannot
//! render to a three-channel target, so the colour target is RGBA8 with its
//! alpha unused, stored as sRGB so its 8 bits hold the same gamma-encoded
//! values the original's RGB8 holds (Exarust shades in linear light).

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::{Core3dSystems, FullscreenShader};
use bevy::image::{Image, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{
    sampler, texture_2d, texture_2d_array, uniform_buffer,
};
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FragmentState, LoadOp,
    Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, SamplerBindingType, ShaderStages, ShaderType, StoreOp,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::texture::{FallbackImage, GpuImage};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::supersample::{DisplayCamera, Offscreen};

/// The composite host's renderer side: the layer-stacking shader library,
/// the bake shader, the bake queue and the render-world bake pass.
pub struct TerrainCompositePlugin;

impl Plugin for TerrainCompositePlugin {
    fn build(&self, app: &mut App) {
        bevy::shader::load_shader_library!(app, "terrain_layers.wesl");
        embedded_asset!(app, "terrain_bake.wesl");
        app.init_resource::<TerrainBakeQueue>()
            .add_plugins(ExtractResourcePlugin::<TerrainBakeQueue>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<PreparedBakes>()
            .add_systems(RenderStartup, init_bake_pipeline)
            .add_systems(
                Render,
                prepare_bakes.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Core3d, run_bakes.in_set(Core3dSystems::Prepass));
    }
}

/// A composite set's side at `level`: `32 << level`. Levels outside 0..=15
/// have no size.
pub fn set_size(level: i32) -> u32 {
    if (0..=15).contains(&level) {
        TEXTURE_BASE << level
    } else {
        0
    }
}

/// Side of a level's set at level 0 (the original's stored 32).
pub const TEXTURE_BASE: u32 = 32;

/// The four set targets' formats: colour (the original's RGB8, here sRGB
/// RGBA8), normal RG8, gloss R8 and the half-size displacement R8.
pub const COLOUR_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
pub const NORMAL_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;
pub const GLOSS_FORMAT: TextureFormat = TextureFormat::R8Unorm;
pub const DISPLACEMENT_FORMAT: TextureFormat = TextureFormat::R8Unorm;

/// Which of the set's four targets the compositor creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetFeatures {
    pub normal: bool,
    pub gloss: bool,
    pub displacement: bool,
}

impl Default for SetFeatures {
    fn default() -> Self {
        Self {
            normal: true,
            gloss: true,
            displacement: true,
        }
    }
}

/// The images of one hosted set, in the original's allocation order.
#[derive(Clone, Debug, PartialEq)]
pub struct HostedSet {
    pub level: i32,
    pub colour: Handle<Image>,
    pub normal: Option<Handle<Image>>,
    pub gloss: Option<Handle<Image>>,
    /// Half the colour target's side.
    pub displacement: Option<Handle<Image>>,
}

impl HostedSet {
    /// The image handles that exist, in allocation order.
    pub fn handles(&self) -> Vec<Handle<Image>> {
        let mut out = vec![self.colour.clone()];
        out.extend(self.normal.clone());
        out.extend(self.gloss.clone());
        out.extend(self.displacement.clone());
        out
    }
}

/// The composite set pool's render targets, by pool id (0..=20 high, 21..=63
/// low).
#[derive(Resource, Debug)]
pub struct TerrainCompositeHost {
    sets: HashMap<u32, HostedSet>,
    enabled: bool,
}

impl Default for TerrainCompositeHost {
    fn default() -> Self {
        Self {
            sets: HashMap::new(),
            enabled: std::env::var("EXA_TERRAIN_SETS").is_ok_and(|v| v == "1"),
        }
    }
}

impl TerrainCompositeHost {
    /// Whether set images are created at all.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Turns allocation on or off. Assignments are still accepted while off;
    /// no image is created.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.sets.clear();
        }
    }

    /// The hosted sets.
    pub fn sets(&self) -> &HashMap<u32, HostedSet> {
        &self.sets
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// The set identified by a pool id, once hosted.
    pub fn set(&self, id: u32) -> Option<&HostedSet> {
        self.sets.get(&id)
    }

    /// Creates a set's images, in the original's order and formats: colour
    /// RGB8, normal RG8, gloss R8 and displacement R8 at half the side. The
    /// images are render targets and sample linearly. An id already hosted
    /// at the same level keeps its images.
    pub fn allocate(
        &mut self,
        images: &mut Assets<Image>,
        id: u32,
        level: i32,
        features: SetFeatures,
    ) -> Option<&HostedSet> {
        if self.sets.contains_key(&id) {
            return self.sets.get(&id);
        }
        let side = set_size(level);
        if side == 0 || !self.enabled {
            return None;
        }
        let mut target = |side: u32, format: TextureFormat| target_image(images, side, format);
        let colour = target(side, COLOUR_FORMAT);
        let normal = features.normal.then(|| target(side, NORMAL_FORMAT));
        let gloss = features.gloss.then(|| target(side, GLOSS_FORMAT));
        let displacement = features
            .displacement
            .then(|| target((side >> 1).max(1), DISPLACEMENT_FORMAT));
        Some(self.sets.entry(id).or_insert(HostedSet {
            level,
            colour,
            normal,
            gloss,
            displacement,
        }))
    }

    /// Drops a set's images (the set went back to its pool).
    pub fn release(&mut self, id: u32) {
        self.sets.remove(&id);
    }

    /// Drops every set: the terrain's release slot, as a level unloads its
    /// composite caches.
    pub fn clear(&mut self) {
        self.sets.clear();
    }

    /// Reconciles the hosted sets with the selector's assignments: `held`
    /// lists every set a block holds as `(id, level, features)` in selection
    /// order. Missing sets are created at most `budget` at a time so a frame
    /// never makes many render targets at once; sets no block holds any more
    /// are released. Returns the ids created this call.
    pub fn sync(
        &mut self,
        images: &mut Assets<Image>,
        held: &[(u32, i32, SetFeatures)],
        budget: usize,
    ) -> Vec<u32> {
        let keep: std::collections::HashSet<u32> = held.iter().map(|&(id, _, _)| id).collect();
        let stale: Vec<u32> = self
            .sets
            .keys()
            .copied()
            .filter(|id| !keep.contains(id))
            .collect();
        for id in stale {
            self.release(id);
        }
        let mut made = Vec::new();
        if !self.enabled {
            return made;
        }
        for &(id, level, features) in held {
            // A set id reused by a new level (or a new texture-detail
            // setting) is rebuilt at its level.
            if self.sets.get(&id).is_some_and(|set| set.level != level) {
                self.release(id);
            }
            if made.len() >= budget {
                break;
            }
            if self.sets.contains_key(&id) {
                continue;
            }
            if self.allocate(images, id, level, features).is_some() {
                made.push(id);
            }
        }
        made
    }
}

// ---- The bake ---------------------------------------------------------------

/// Layers a bake (and the terrain draw) stacks at most.
pub const BAKE_LAYERS: usize = 16;

/// The bake's uniform, as `terrain_bake.wesl` declares it: the layers'
/// values exactly as the terrain draw's uniform holds them, the block's low
/// world corner and side, and its window on the tile's vertex coordinates.
#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub struct TerrainBakeParams {
    /// Linear RGB multiplier per layer; alpha its cover strength.
    pub colour: [Vec4; BAKE_LAYERS],
    /// Mask sharpness `s`, `t`, 1 when masked, repeats per metre.
    pub blend: [Vec4; BAKE_LAYERS],
    /// Normal-pass sharpness `s`, `t`, normal strength, gloss.
    pub normal: [Vec4; BAKE_LAYERS],
    /// Gloss factor and the parallax height's base, reference and scale.
    pub surface: [Vec4; BAKE_LAYERS],
    /// The block's low world corner x, z (metres) and its side (metres).
    pub block: Vec4,
    /// The block's low corner on the tile's 0–1 vertex coordinates and its
    /// span on them.
    pub tile: Vec4,
    /// Layers in use.
    pub layers: u32,
    pub pad1: u32,
    pub pad2: u32,
    pub pad3: u32,
}

/// One block's bake into one set: its layers' textures (diffuse, packed
/// surface, normal; a missing one reads the renderer's white fallback, as
/// the draw's material does), the tile's per-vertex covers, the image whose
/// sampler the layers repeat through, the uniform and the set's targets.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainBake {
    pub set: u32,
    pub targets: HostedSet,
    pub params: TerrainBakeParams,
    pub layer_sampler: Handle<Image>,
    pub covers: Handle<Image>,
    pub layers: Vec<[Option<Handle<Image>>; 3]>,
}

/// One queued bake with its ticket.
#[derive(Clone, Debug)]
struct QueuedBake {
    ticket: u64,
    bake: TerrainBake,
}

/// The bakes waiting for the render world. A bake stays queued until the
/// render world reports its ticket done (its targets and textures may not
/// be on the GPU yet the first frames); [`TerrainBakeQueue::finished`]
/// takes the done tickets out.
#[derive(Resource, Clone, Default, ExtractResource)]
#[extract_app(RenderApp)]
pub struct TerrainBakeQueue {
    jobs: Vec<QueuedBake>,
    next: u64,
    done: Arc<Mutex<Vec<u64>>>,
}

impl TerrainBakeQueue {
    /// Queues a bake and returns its ticket.
    pub fn push(&mut self, bake: TerrainBake) -> u64 {
        self.next += 1;
        self.jobs.push(QueuedBake {
            ticket: self.next,
            bake,
        });
        self.next
    }

    /// Drops a queued bake (its block lost the set before it ran).
    pub fn cancel(&mut self, ticket: u64) {
        self.jobs.retain(|job| job.ticket != ticket);
    }

    /// Drops every queued bake (a level unloads); tickets keep counting so
    /// a new bake never reuses one the render world has seen.
    pub fn clear(&mut self) {
        self.jobs.clear();
    }

    /// Whether a bake is still queued.
    pub fn pending(&self, ticket: u64) -> bool {
        self.jobs.iter().any(|job| job.ticket == ticket)
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// The tickets the render world finished since the last call; their
    /// bakes leave the queue.
    pub fn finished(&mut self) -> Vec<u64> {
        let done: Vec<u64> = self
            .done
            .lock()
            .map(|mut done| std::mem::take(&mut *done))
            .unwrap_or_default();
        if !done.is_empty() {
            self.jobs.retain(|job| !done.contains(&job.ticket));
        }
        done
    }

    /// Reports a ticket done, as the render world does after the passes.
    pub fn report_done(&self, ticket: u64) {
        if let Ok(mut done) = self.done.lock() {
            done.push(ticket);
        }
    }
}

/// The compositor's four passes in its order, with the fragment entry each
/// runs and the target format it writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BakePass {
    Normal,
    Colour,
    Gloss,
    Displacement,
}

impl BakePass {
    /// The driver's order: normal, colour, gloss, displacement.
    pub const ORDER: [BakePass; 4] = [
        BakePass::Normal,
        BakePass::Colour,
        BakePass::Gloss,
        BakePass::Displacement,
    ];

    pub fn entry(self) -> &'static str {
        match self {
            BakePass::Normal => "bake_normal",
            BakePass::Colour => "bake_colour",
            BakePass::Gloss => "bake_gloss",
            BakePass::Displacement => "bake_height",
        }
    }

    pub fn format(self) -> TextureFormat {
        match self {
            BakePass::Normal => NORMAL_FORMAT,
            BakePass::Colour => COLOUR_FORMAT,
            BakePass::Gloss => GLOSS_FORMAT,
            BakePass::Displacement => DISPLACEMENT_FORMAT,
        }
    }

    /// The set's target this pass writes, when the set has it.
    pub fn target(self, set: &HostedSet) -> Option<&Handle<Image>> {
        match self {
            BakePass::Normal => set.normal.as_ref(),
            BakePass::Colour => Some(&set.colour),
            BakePass::Gloss => set.gloss.as_ref(),
            BakePass::Displacement => set.displacement.as_ref(),
        }
    }
}

/// The bake's binding numbers: the uniform, the layer sampler, the covers
/// and their sampler, then each layer's diffuse, surface and normal.
pub mod bake_bindings {
    pub const PARAMS: u32 = 0;
    pub const LAYER_SAMPLER: u32 = 1;
    pub const COVERS: u32 = 2;
    pub const COVER_SAMPLER: u32 = 3;
    pub const DIFFUSE: u32 = 4;
    pub const SURFACE: u32 = DIFFUSE + super::BAKE_LAYERS as u32;
    pub const NORMAL: u32 = SURFACE + super::BAKE_LAYERS as u32;
    /// One past the last binding.
    pub const END: u32 = NORMAL + super::BAKE_LAYERS as u32;
}

/// The bake's bind group layout entries, in binding order.
fn bake_layout_entries() -> Vec<BindGroupLayoutEntry> {
    use bake_bindings::*;
    let stages = ShaderStages::FRAGMENT;
    let float = TextureSampleType::Float { filterable: true };
    let mut entries = vec![
        uniform_buffer::<TerrainBakeParams>(false).build(PARAMS, stages),
        sampler(SamplerBindingType::Filtering).build(LAYER_SAMPLER, stages),
        texture_2d_array(float).build(COVERS, stages),
        sampler(SamplerBindingType::Filtering).build(COVER_SAMPLER, stages),
    ];
    entries.extend((DIFFUSE..END).map(|binding| texture_2d(float).build(binding, stages)));
    entries
}

#[derive(Resource)]
struct BakePipeline {
    layout: BindGroupLayoutDescriptor,
    /// One pipeline per pass, in [`BakePass::ORDER`].
    passes: [CachedRenderPipelineId; 4],
}

fn init_bake_pipeline(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new("exa_terrain_bake", &bake_layout_entries());
    let shader: Handle<Shader> = assets.load(
        AssetPath::from_path_buf(embedded_path!("terrain_bake.wesl")).with_source("embedded"),
    );
    let vertex = fullscreen.to_vertex_state();
    let passes = BakePass::ORDER.map(|pass| {
        cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some(format!("exa_terrain_{}", pass.entry()).into()),
            layout: vec![layout.clone()],
            vertex: vertex.clone(),
            fragment: Some(FragmentState {
                shader: shader.clone(),
                entry_point: Some(pass.entry().into()),
                // Each pass overwrites its target: the original's groups of
                // four additive layers are folded into one stack.
                targets: vec![Some(ColorTargetState {
                    format: pass.format(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        })
    });
    commands.insert_resource(BakePipeline { layout, passes });
}

/// A bake whose inputs are all on the GPU, ready for this frame's passes.
struct ReadyBake {
    ticket: u64,
    group: BindGroup,
    /// Per pass in [`BakePass::ORDER`], the target view (None: the set has
    /// no such target).
    targets: [Option<TextureView>; 4],
    _params: UniformBuffer<TerrainBakeParams>,
}

#[derive(Resource, Default)]
struct PreparedBakes {
    ready: Option<ReadyBake>,
    /// Tickets already baked, so a queue extracted again before the main
    /// world takes them out is not baked twice.
    baked: HashSet<u64>,
}

/// Prepares the first queued bake whose targets and textures are all on the
/// GPU: one block a frame, as the original's selector bakes one a frame.
#[allow(clippy::too_many_arguments)]
fn prepare_bakes(
    queue: Res<TerrainBakeQueue>,
    pipeline: Option<Res<BakePipeline>>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    fallback: Res<FallbackImage>,
    device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut prepared: ResMut<PreparedBakes>,
) {
    prepared.ready = None;
    let live: HashSet<u64> = queue.jobs.iter().map(|job| job.ticket).collect();
    prepared.baked.retain(|ticket| live.contains(ticket));
    let Some(pipeline) = pipeline else {
        return;
    };
    if pipeline
        .passes
        .iter()
        .any(|&id| cache.get_render_pipeline(id).is_none())
    {
        return;
    }
    for job in &queue.jobs {
        if prepared.baked.contains(&job.ticket) {
            continue;
        }
        let bake = &job.bake;
        let image = |handle: &Handle<Image>| images.get(handle);
        let (Some(layer_sampler), Some(covers)) = (image(&bake.layer_sampler), image(&bake.covers))
        else {
            continue;
        };
        let mut views: Vec<&TextureView> = Vec::with_capacity(3 * BAKE_LAYERS);
        let mut missing = false;
        for slot in 0..3 {
            for k in 0..BAKE_LAYERS {
                let texture = bake.layers.get(k).and_then(|layer| layer[slot].as_ref());
                match texture {
                    Some(handle) => match image(handle) {
                        Some(gpu) => views.push(&gpu.texture_view),
                        None => missing = true,
                    },
                    None => views.push(&fallback.d2.texture_view),
                }
            }
        }
        let targets = BakePass::ORDER.map(|pass| {
            pass.target(&bake.targets)
                .and_then(image)
                .map(|gpu| gpu.texture_view.clone())
        });
        if missing || targets[1].is_none() {
            continue;
        }
        let mut params = UniformBuffer::from(bake.params);
        params.write_buffer(&device, &render_queue);
        let Some(binding) = params.binding() else {
            continue;
        };
        let mut entries = vec![
            BindGroupEntry {
                binding: bake_bindings::PARAMS,
                resource: binding,
            },
            BindGroupEntry {
                binding: bake_bindings::LAYER_SAMPLER,
                resource: BindingResource::Sampler(&layer_sampler.sampler),
            },
            BindGroupEntry {
                binding: bake_bindings::COVERS,
                resource: BindingResource::TextureView(&covers.texture_view),
            },
            BindGroupEntry {
                binding: bake_bindings::COVER_SAMPLER,
                resource: BindingResource::Sampler(&covers.sampler),
            },
        ];
        entries.extend(views.iter().enumerate().map(|(i, view)| BindGroupEntry {
            binding: bake_bindings::DIFFUSE + i as u32,
            resource: BindingResource::TextureView(view),
        }));
        let group = device.create_bind_group(
            "exa_terrain_bake",
            &cache.get_bind_group_layout(&pipeline.layout),
            &entries,
        );
        drop(entries);
        prepared.ready = Some(ReadyBake {
            ticket: job.ticket,
            group,
            targets,
            _params: params,
        });
        return;
    }
}

/// Runs the prepared bake's passes once a frame, by the scene view, and
/// reports its ticket done.
fn run_bakes(
    view: ViewQuery<(Has<DisplayCamera>, Has<Offscreen>)>,
    queue: Res<TerrainBakeQueue>,
    pipeline: Option<Res<BakePipeline>>,
    cache: Res<PipelineCache>,
    mut prepared: ResMut<PreparedBakes>,
    mut ctx: RenderContext,
) {
    let (display, offscreen) = view.into_inner();
    if display || offscreen {
        return;
    }
    let Some(pipeline) = pipeline else {
        return;
    };
    let Some(ready) = prepared.ready.take() else {
        return;
    };
    for (index, pass) in BakePass::ORDER.into_iter().enumerate() {
        let (Some(target), Some(render_pipeline)) = (
            ready.targets[index].as_ref(),
            cache.get_render_pipeline(pipeline.passes[index]),
        ) else {
            continue;
        };
        let mut render_pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some(pass.entry()),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(default()),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        render_pass.set_pipeline(render_pipeline);
        render_pass.set_bind_group(0, &ready.group, &[]);
        render_pass.draw(0..3, 0..1);
    }
    prepared.baked.insert(ready.ticket);
    queue.report_done(ready.ticket);
}

/// A render-target image of `side`² with the given format, sampled linearly.
fn target_image(images: &mut Assets<Image>, side: u32, format: TextureFormat) -> Handle<Image> {
    let mut image = Image::new_uninit(
        Extent3d {
            width: side.max(1),
            height: side.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::RENDER_ATTACHMENT
        | TextureUsages::COPY_SRC
        | TextureUsages::COPY_DST;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    images.add(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> TerrainCompositeHost {
        let mut host = TerrainCompositeHost::default();
        host.set_enabled(true);
        host
    }

    #[test]
    fn a_set_is_created_at_the_originals_sizes_and_formats() {
        let mut host = host();
        let mut images = Assets::<Image>::default();
        let set = host
            .allocate(&mut images, 0, 5, SetFeatures::default())
            .expect("a level-5 set")
            .clone();
        // A low set is 512² with a 256² displacement target.
        let low = host
            .allocate(&mut images, 21, 4, SetFeatures::default())
            .unwrap()
            .clone();
        assert_eq!(set.level, 5);
        let size = |handle: &Handle<Image>| {
            let image = images.get(handle).unwrap();
            (
                image.texture_descriptor.size.width,
                image.texture_descriptor.size.height,
                image.texture_descriptor.format,
            )
        };
        assert_eq!(
            size(&set.colour),
            (1024, 1024, TextureFormat::Rgba8UnormSrgb)
        );
        assert_eq!(
            size(set.normal.as_ref().unwrap()),
            (1024, 1024, TextureFormat::Rg8Unorm)
        );
        assert_eq!(
            size(set.gloss.as_ref().unwrap()),
            (1024, 1024, TextureFormat::R8Unorm)
        );
        assert_eq!(
            size(set.displacement.as_ref().unwrap()),
            (512, 512, TextureFormat::R8Unorm)
        );
        let image = images.get(&set.colour).unwrap();
        assert!(
            image
                .texture_descriptor
                .usage
                .contains(TextureUsages::RENDER_ATTACHMENT)
        );
        assert!(
            image
                .texture_descriptor
                .usage
                .contains(TextureUsages::TEXTURE_BINDING)
        );
        assert_eq!(size(&low.colour), (512, 512, TextureFormat::Rgba8UnormSrgb));
        assert_eq!(
            size(low.displacement.as_ref().unwrap()),
            (256, 256, TextureFormat::R8Unorm)
        );
        assert_eq!(host.len(), 2);
        assert_eq!(set_size(0), 32);
        assert_eq!(set_size(5), 1024);
        assert_eq!(set_size(-1), 0);
        assert_eq!(set_size(16), 0);
    }

    #[test]
    fn disabled_features_leave_their_targets_out() {
        let mut host = host();
        let mut images = Assets::<Image>::default();
        let set = host
            .allocate(
                &mut images,
                7,
                4,
                SetFeatures {
                    normal: false,
                    gloss: true,
                    displacement: false,
                },
            )
            .unwrap();
        assert!(set.normal.is_none());
        assert!(set.gloss.is_some());
        assert!(set.displacement.is_none());
        assert_eq!(set.handles().len(), 2);
    }

    #[test]
    fn the_sync_creates_a_budget_a_frame_and_releases_stale_sets() {
        let mut host = host();
        let mut images = Assets::<Image>::default();
        let held = [
            (0, 5, SetFeatures::default()),
            (21, 4, SetFeatures::default()),
            (1, 5, SetFeatures::default()),
        ];
        // One new set a frame.
        assert_eq!(host.sync(&mut images, &held, 1), [0]);
        assert_eq!(host.len(), 1);
        assert_eq!(host.sync(&mut images, &held, 1), [21]);
        assert_eq!(host.sync(&mut images, &held, 1), [1]);
        // Nothing new now.
        assert!(host.sync(&mut images, &held, 1).is_empty());
        // A set whose level changed is rebuilt at the new level.
        let relaid = [(0u32, 4i32, SetFeatures::default())];
        assert_eq!(host.sync(&mut images, &relaid, 1), [0]);
        assert_eq!(host.set(0).unwrap().level, 4);
        assert_eq!(host.len(), 1);
        // The same set held again at the original level is rebuilt again.
        assert_eq!(host.sync(&mut images, &held, 1), [0]);
        assert_eq!(host.set(0).unwrap().level, 5);
        // A set no block holds goes back.
        let fewer = [held[0]];
        assert!(host.sync(&mut images, &fewer, 1).is_empty());
        assert!(host.set(21).is_none() && host.set(1).is_none());
        assert!(host.set(0).is_some());
        // Clearing drops everything too.
        host.clear();
        assert!(host.is_empty());
        assert!(host.sync(&mut images, &held, 1) == [0]);
        // Disabling drops everything.
        host.set_enabled(false);
        assert!(host.is_empty());
        assert!(host.sync(&mut images, &held, 1).is_empty());
        assert!(host.is_empty());
    }

    /// The uniform's WGSL struct, cut out of the bake shader and laid out by
    /// naga on its own (the shader itself imports Bevy's fullscreen vertex,
    /// which `tests/surface_shader.rs` composes).
    fn wgsl_struct_size(source: &str, name: &str) -> u64 {
        let start = source
            .find(&format!("struct {name} {{"))
            .unwrap_or_else(|| panic!("{name} is declared"));
        let end = start + source[start..].find("\n}").expect("the struct closes") + 2;
        let module_source = format!(
            "{}\n@group(0) @binding(0) var<uniform> probe: {name};\n",
            &source[start..end]
        );
        let module = naga::front::wgsl::parse_str(&module_source).expect("the struct parses");
        let mut layouter = naga::proc::Layouter::default();
        layouter
            .update(module.to_ctx())
            .expect("the struct lays out");
        let (handle, _) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .expect("the struct is a type");
        u64::from(layouter[handle].size)
    }

    #[test]
    fn the_bake_uniform_matches_the_shaders_layout() {
        let size = wgsl_struct_size(include_str!("terrain_bake.wesl"), "TerrainBakeParams");
        // Four arrays of 16 vec4, two vec4 and four words.
        assert_eq!(size, 4 * 16 * 16 + 2 * 16 + 16);
        assert_eq!(size, <TerrainBakeParams as ShaderType>::min_size().get());
    }

    #[test]
    fn the_bake_binds_every_layer_texture_in_order() {
        let entries = bake_layout_entries();
        assert_eq!(entries.len(), 4 + 3 * BAKE_LAYERS);
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(entry.binding, i as u32);
        }
        assert_eq!(bake_bindings::SURFACE, 20);
        assert_eq!(bake_bindings::NORMAL, 36);
        assert_eq!(bake_bindings::END, 52);
        // The shader declares the same bindings.
        let source = include_str!("terrain_bake.wesl");
        for k in 0..BAKE_LAYERS as u32 {
            for (name, base) in [
                ("diffuse", bake_bindings::DIFFUSE),
                ("surface", bake_bindings::SURFACE),
                ("normal", bake_bindings::NORMAL),
            ] {
                let line = format!(
                    "@group(0) @binding({}) var {name}{k}: texture_2d<f32>;",
                    base + k
                );
                assert!(source.contains(&line), "{line}");
            }
        }
    }

    #[test]
    fn the_passes_run_in_the_drivers_order_into_their_targets() {
        let mut host = host();
        let mut images = Assets::<Image>::default();
        let set = host
            .allocate(
                &mut images,
                3,
                4,
                SetFeatures {
                    normal: true,
                    gloss: false,
                    displacement: true,
                },
            )
            .unwrap()
            .clone();
        let entries: Vec<_> = BakePass::ORDER.iter().map(|p| p.entry()).collect();
        assert_eq!(
            entries,
            ["bake_normal", "bake_colour", "bake_gloss", "bake_height"]
        );
        // Each pass writes the target of its own format, and a disabled
        // target is skipped.
        for pass in BakePass::ORDER {
            match pass.target(&set) {
                Some(handle) => {
                    assert_eq!(
                        images.get(handle).unwrap().texture_descriptor.format,
                        pass.format()
                    );
                }
                None => assert_eq!(pass, BakePass::Gloss),
            }
        }
    }

    #[test]
    fn a_bake_stays_queued_until_the_render_world_reports_it() {
        let mut queue = TerrainBakeQueue::default();
        let bake = TerrainBake {
            set: 0,
            targets: HostedSet {
                level: 4,
                colour: Handle::default(),
                normal: None,
                gloss: None,
                displacement: None,
            },
            params: TerrainBakeParams::default(),
            layer_sampler: Handle::default(),
            covers: Handle::default(),
            layers: Vec::new(),
        };
        let first = queue.push(bake.clone());
        let second = queue.push(bake);
        assert_eq!((first, second), (1, 2));
        assert!(queue.finished().is_empty());
        assert_eq!(queue.len(), 2);
        // The render world's copy shares the report.
        let extracted = queue.clone();
        extracted.report_done(first);
        assert_eq!(queue.finished(), [first]);
        assert!(!queue.pending(first) && queue.pending(second));
        queue.cancel(second);
        assert!(queue.is_empty());
    }
}
