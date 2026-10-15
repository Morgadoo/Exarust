//! Cached cube shadow maps for static casters, as the original keeps its
//! shadow maps. A light's cube face is rendered only when it is needed (it
//! looks into the camera's view) and dirty (the light or a caster linked to
//! it moved, or the light just took its slot). Afterwards it stays as it is,
//! frame after frame: a still dungeon renders almost no cube faces. Torch
//! flicker changes a light's colour, not its position, and never dirties it.
//!
//! The faces hold the original's linear depth, distance / (R·k_c), R the
//! light's radius and k_c the caster's depth scale from its shadow-bias
//! level. They live in a pool of cube textures shared by every light; a
//! light that needs a slot takes a free one or one whose light needs none
//! this frame. Skinned meshes (characters) keep casting through Bevy's own
//! cube maps; the shaders combine both. Rules: `docs/RENDERING.md` (Cached
//! cube shadows).

use std::ops::Range;
use std::sync::{Arc, Mutex};

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path, uuid_handle};
use bevy::camera::primitives::{Aabb, CUBE_MAP_FACES, Frustum};
use bevy::camera::visibility::VisibilitySystems;
use bevy::core_pipeline::Core3dSystems;
use bevy::core_pipeline::schedule::Core3d;
use bevy::image::{ImageCompareFunction, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::Affine3A;
use bevy::mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::mesh::{RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{
    sampler, storage_buffer_read_only_sized, texture_2d,
};
use bevy::render::render_resource::{
    BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BufferUsages,
    CachedRenderPipelineId, CompareFunction, DepthStencilState, Extent3d, Face, FragmentState,
    LoadOp, Operations, PipelineCache, PrimitiveState, RawBufferVec,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
    SamplerBindingType, ShaderStages, SpecializedMeshPipeline, SpecializedMeshPipelineError,
    SpecializedMeshPipelines, StoreOp, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use bytemuck::{Pod, Zeroable};

use crate::lights::{LightsSystems, ShadowBias};
use crate::material::{ExaMaterial, flags};
use crate::supersample::{DisplayCamera, SceneCamera};

pub struct CubeShadowsPlugin;

impl Plugin for CubeShadowsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "cube_shadow.wesl");
        app.init_resource::<PointShadows>()
            .init_resource::<Pool>()
            .init_resource::<CubeJobs>()
            .add_plugins(ExtractResourcePlugin::<CubeJobs>::default())
            .add_systems(Startup, create_pool)
            .add_systems(
                PostUpdate,
                add_cube_shadows
                    .in_set(LightsSystems::Ids)
                    .after(TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                (apply_failures, needed_faces, assign_slots, build_jobs)
                    .chain()
                    .in_set(LightsSystems::Cubes)
                    .after(VisibilitySystems::UpdateFrusta),
            );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedMeshPipelines<CubePipeline>>()
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(
                Render,
                prepare_draws.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Core3d, render_faces.in_set(Core3dSystems::Prepass));
    }
}

use bevy::transform::TransformSystems;

/// The pool of cube textures, bound to every surface material.
pub const CUBE_POOL: Handle<Image> = uuid_handle!("6b1f3c2e-9a4d-4f7b-8e21-5c0d7a9b3e14");
/// Texels per face side (the original's at its top shadow detail).
pub const CUBE_SIZE: u32 = crate::POINT_SHADOW_SIZE as u32;
/// Cubes in the pool: the most lights that can need a cube at once.
pub const CUBE_CAPACITY: u32 = 64;
/// The faces' near plane, metres (the original's 3 cm).
const NEAR: f32 = 0.03;
/// A light's Bevy range over its radius R.
const REACH: f32 = 1.25;
const ALL_FACES: u8 = 0x3F;

/// Whether point lights cast shadows at all (`--no-shadows` turns it off).
#[derive(Resource, Clone, Copy, Debug)]
pub struct PointShadows(pub bool);

impl Default for PointShadows {
    fn default() -> Self {
        Self(true)
    }
}

/// A light's cached cube: its pool slot and the faces to render.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CubeShadow {
    slot: Option<u32>,
    /// Faces whose content is out of date.
    dirty: u8,
    /// Faces looking into the camera's view this frame.
    needed: u8,
}

impl CubeShadow {
    /// Marks every face out of date (the light or one of its casters moved).
    pub fn invalidate(&mut self) {
        self.dirty = ALL_FACES;
    }
}

/// The static meshes casting into a light's cached cube, kept by the world's
/// shadow links.
#[derive(Component, Clone, Debug, Default)]
pub struct StaticCasters(pub Vec<Entity>);

/// A light's cube for the shaders: its pool slot and its radius R (metres).
/// Changes only when the slot does.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CubeSlot {
    pub slot: Option<u32>,
    pub radius: f32,
}

/// Which light holds each slot.
#[derive(Resource)]
struct Pool {
    owners: Vec<Option<Entity>>,
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            owners: vec![None; CUBE_CAPACITY as usize],
        }
    }
}

/// Faces the render world could not draw (a mesh or pipeline not ready),
/// handed back so they stay dirty.
#[derive(Clone, Default)]
struct Failed(Arc<Mutex<Vec<(Entity, u8)>>>);

/// This frame's faces to render, for the render world.
#[derive(Resource, Clone, Default, ExtractResource)]
#[extract_app(RenderApp)]
struct CubeJobs {
    faces: Vec<FaceJob>,
    draws: Vec<CasterDraw>,
    failed: Failed,
}

#[derive(Clone, Debug)]
struct FaceJob {
    light: Entity,
    face: u8,
    /// The face's layer in the pool.
    layer: u32,
    draws: Range<u32>,
}

#[derive(Clone, Debug)]
struct CasterDraw {
    mesh: AssetId<Mesh>,
    gpu: GpuDraw,
    two_sided: bool,
    /// The texture an alpha-cut material cuts its shadow with.
    cut: Option<AssetId<Image>>,
}

/// One draw as the caster shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GpuDraw {
    world_from_local: [f32; 16],
    clip_from_world: [f32; 16],
    light: [f32; 4],
    alpha: [f32; 4],
}

fn create_pool(mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_uninit(
        Extent3d {
            width: CUBE_SIZE,
            height: CUBE_SIZE,
            depth_or_array_layers: 6 * CUBE_CAPACITY,
        },
        TextureDimension::D2,
        // The original's 16-bit depth faces.
        TextureFormat::Depth16Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::CubeArray),
        ..default()
    });
    // Hardware 2 × 2 comparison per tap, lit when the reference is less.
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        compare: Some(ImageCompareFunction::Less),
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    let _ = images.insert(CUBE_POOL.id(), image);
}

type NewLights<'w, 's> = Query<'w, 's, (Entity, &'static PointLight), Without<CubeShadow>>;

/// Gives new lights a cube; a moved light's faces go out of date.
fn add_cube_shadows(
    mut commands: Commands,
    new: NewLights,
    mut moved: Query<&mut CubeShadow, Changed<GlobalTransform>>,
) {
    for (entity, light) in &new {
        commands.entity(entity).insert((
            CubeShadow {
                dirty: ALL_FACES,
                ..default()
            },
            StaticCasters::default(),
            CubeSlot {
                slot: None,
                radius: light.range / REACH,
            },
        ));
    }
    for mut cube in &mut moved {
        cube.invalidate();
    }
}

fn apply_failures(jobs: Res<CubeJobs>, mut cubes: Query<&mut CubeShadow>) {
    let failed: Vec<(Entity, u8)> = jobs
        .failed
        .0
        .lock()
        .map(|mut f| std::mem::take(&mut *f))
        .unwrap_or_default();
    for (light, faces) in failed {
        if let Ok(mut cube) = cubes.get_mut(light) {
            cube.dirty |= faces;
        }
    }
}

/// The face's view rotation and its far-corner box, as Bevy builds its cube
/// faces (the shaders read the cube with Z flipped to match).
fn face_box(position: Vec3, radius: f32, face: usize) -> Aabb {
    let f = &CUBE_MAP_FACES[face];
    let side = f.target.cross(f.up);
    let mut lo = position;
    let mut hi = position;
    for (a, b) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let p = position + (f.target + f.up * a + side * b) * radius;
        lo = lo.min(p);
        hi = hi.max(p);
    }
    Aabb::from_min_max(lo, hi)
}

/// Marks the faces that look into the camera's view.
fn needed_faces(
    enabled: Res<PointShadows>,
    cameras: Query<&Frustum, SceneCamera>,
    mut lights: Query<(&GlobalTransform, &CubeSlot, &StaticCasters, &mut CubeShadow)>,
) {
    let frusta: Vec<&Frustum> = cameras.iter().collect();
    for (transform, slot, casters, mut cube) in &mut lights {
        let mut needed = 0;
        if enabled.0 && !casters.0.is_empty() {
            for face in 0..6 {
                let b = face_box(transform.translation(), slot.radius, face);
                if frusta
                    .iter()
                    .any(|f| f.intersects_obb(&b, &Affine3A::IDENTITY, true, true))
                {
                    needed |= 1 << face;
                }
            }
        }
        if cube.needed != needed {
            cube.needed = needed;
        }
    }
}

/// Gives lights that need faces a slot: a free one, or one whose light needs
/// none this frame. A light taking a slot renders all six faces.
fn assign_slots(
    mut pool: ResMut<Pool>,
    mut lights: Query<(Entity, &StaticCasters, &mut CubeShadow, &mut CubeSlot)>,
) {
    // Release the slots of lights gone or left without casters.
    for owner in pool.owners.iter_mut() {
        if let Some(e) = *owner {
            match lights.get(e) {
                Ok((_, casters, _, _)) if !casters.0.is_empty() => {}
                _ => *owner = None,
            }
        }
    }
    let mut wanting = Vec::new();
    for (entity, casters, mut cube, mut slot) in &mut lights {
        if casters.0.is_empty() {
            if cube.slot.take().is_some() {
                slot.slot = None;
            }
            continue;
        }
        if cube.slot.is_none() && cube.needed != 0 {
            wanting.push(entity);
        }
    }
    for entity in wanting {
        let free = pool.owners.iter().position(Option::is_none).or_else(|| {
            pool.owners.iter().position(|o| {
                o.and_then(|e| lights.get(e).ok())
                    .is_some_and(|(_, _, c, _)| c.needed == 0)
            })
        });
        let Some(s) = free else { break };
        if let Some(previous) = pool.owners[s].replace(entity)
            && let Ok((_, _, mut cube, mut slot)) = lights.get_mut(previous)
        {
            cube.slot = None;
            cube.invalidate();
            slot.slot = None;
        }
        if let Ok((_, _, mut cube, mut slot)) = lights.get_mut(entity) {
            cube.slot = Some(s as u32);
            cube.invalidate();
            cube.needed = ALL_FACES;
            slot.slot = Some(s as u32);
        }
    }
}

/// The depth scale k_c of a caster of shadow-bias level `v`.
fn caster_scale(v: u32) -> f32 {
    if v == 0 {
        1.0
    } else {
        1.0 - 0.005 * v as f32 - 0.01
    }
}

type CasterData<'w, 's> = Query<
    'w,
    's,
    (
        &'static Mesh3d,
        &'static GlobalTransform,
        Option<&'static ShadowBias>,
        Option<&'static MeshMaterial3d<ExaMaterial>>,
    ),
>;

/// Lists the casters of every face both needed and dirty, for the render
/// world, and marks those faces done.
fn build_jobs(
    mut jobs: ResMut<CubeJobs>,
    mut lights: Query<(
        Entity,
        &GlobalTransform,
        &CubeSlot,
        &StaticCasters,
        &mut CubeShadow,
    )>,
    casters: CasterData,
    materials: Res<Assets<ExaMaterial>>,
) {
    jobs.faces.clear();
    jobs.draws.clear();
    let projection =
        bevy::math::proj::perspective_infinite_reverse(std::f32::consts::FRAC_PI_2, 1.0, NEAR);
    for (entity, transform, slot, list, mut cube) in &mut lights {
        let Some(s) = cube.slot else { continue };
        let faces = cube.needed & cube.dirty;
        if faces == 0 {
            continue;
        }
        cube.dirty &= !faces;
        let position = transform.translation();
        for face in 0..6u8 {
            if faces & (1 << face) == 0 {
                continue;
            }
            let f = &CUBE_MAP_FACES[face as usize];
            let rotation = Transform::IDENTITY.looking_at(f.target, f.up).rotation;
            let world_from_view = Mat4::from_rotation_translation(rotation, position);
            let clip_from_world = projection * world_from_view.inverse();
            let start = jobs.draws.len() as u32;
            for &caster in &list.0 {
                let Ok((mesh, at, bias, material)) = casters.get(caster) else {
                    continue;
                };
                let k_c = caster_scale(bias.map_or(0, |b| b.0));
                let material = material.and_then(|m| materials.get(&m.0));
                let cut = material.and_then(|m| {
                    matches!(m.alpha_mode, AlphaMode::Mask(_)).then(|| {
                        let opacity = m.params.flags & flags::OPACITY_MAP != 0;
                        let texture = if opacity {
                            m.opacity_map.as_ref()
                        } else {
                            m.diffuse_map.as_ref()
                        };
                        texture.map(|t| (t.id(), opacity, m.params.diffuse.w))
                    })?
                });
                jobs.draws.push(CasterDraw {
                    mesh: mesh.0.id(),
                    gpu: GpuDraw {
                        world_from_local: at.to_matrix().to_cols_array(),
                        clip_from_world: clip_from_world.to_cols_array(),
                        light: position.extend(1.0 / (slot.radius * k_c)).to_array(),
                        alpha: [
                            cut.map_or(1.0, |c| c.2),
                            cut.map_or(0.0, |c| if c.1 { 1.0 } else { 0.0 }),
                            0.0,
                            0.0,
                        ],
                    },
                    two_sided: material.is_some_and(|m| m.params.flags & flags::TWO_SIDED != 0),
                    cut: cut.map(|c| c.0),
                });
            }
            let end = jobs.draws.len() as u32;
            jobs.faces.push(FaceJob {
                light: entity,
                face,
                layer: s * 6 + face as u32,
                draws: start..end,
            });
        }
    }
}

// ---- Render world ---------------------------------------------------------

#[derive(Resource, Clone)]
struct CubePipeline {
    draws: BindGroupLayoutDescriptor,
    cut: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CubePipelineKey {
    alpha_cut: bool,
    two_sided: bool,
}

impl SpecializedMeshPipeline for CubePipeline {
    type Key = CubePipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut attributes = vec![Mesh::ATTRIBUTE_POSITION.at_shader_location(0)];
        let mut defs = Vec::new();
        let mut layouts = vec![self.draws.clone()];
        if key.alpha_cut {
            attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(1));
            defs.push("ALPHA_CUT".into());
            layouts.push(self.cut.clone());
        }
        let buffer = layout.0.get_layout(&attributes)?;
        Ok(RenderPipelineDescriptor {
            label: Some("exa_cube_shadow".into()),
            layout: layouts,
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: defs.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![buffer],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: defs,
                entry_point: Some("fragment".into()),
                targets: vec![],
                ..default()
            }),
            // Front faces cast, as the original culls back faces; two-sided
            // materials cast both.
            primitive: PrimitiveState {
                cull_mode: (!key.two_sided).then_some(Face::Back),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth16Unorm,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::LessEqual),
                stencil: default(),
                bias: default(),
            }),
            ..default()
        })
    }
}

fn init_pipeline(mut commands: Commands, assets: Res<AssetServer>) {
    let draws = BindGroupLayoutDescriptor::new(
        "exa_cube_draws",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            storage_buffer_read_only_sized(false, None),
        ),
    );
    let cut = BindGroupLayoutDescriptor::new(
        "exa_cube_cut",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    commands.insert_resource(CubePipeline {
        draws,
        cut,
        shader: assets.load(
            AssetPath::from_path_buf(embedded_path!("cube_shadow.wesl")).with_source("embedded"),
        ),
    });
}

/// The frame's draws on the GPU.
#[derive(Resource)]
struct PreparedCubes {
    buffer: RawBufferVec<GpuDraw>,
    draws: Option<BindGroup>,
    cuts: HashMap<AssetId<Image>, BindGroup>,
    /// Per draw, its pipeline.
    pipelines: Vec<Option<CachedRenderPipelineId>>,
    /// Per pool layer, a view to render into.
    layers: Vec<TextureView>,
    /// Faces left for this frame (rendered once, by the first scene view).
    pending: bool,
}

#[allow(clippy::too_many_arguments)]
fn prepare_draws(
    mut commands: Commands,
    jobs: Res<CubeJobs>,
    pipeline: Option<Res<CubePipeline>>,
    mut specialized: ResMut<SpecializedMeshPipelines<CubePipeline>>,
    cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    prepared: Option<ResMut<PreparedCubes>>,
) {
    let Some(pipeline) = pipeline else { return };
    let mut prepared = match prepared {
        Some(p) => p,
        None => {
            commands.insert_resource(PreparedCubes {
                buffer: RawBufferVec::new(BufferUsages::STORAGE),
                draws: None,
                cuts: HashMap::default(),
                pipelines: Vec::new(),
                layers: Vec::new(),
                pending: false,
            });
            return;
        }
    };
    let Some(pool) = images.get(CUBE_POOL.id()) else {
        return;
    };
    if prepared.layers.len() != (6 * CUBE_CAPACITY) as usize {
        prepared.layers = (0..6 * CUBE_CAPACITY)
            .map(|layer| {
                pool.texture.create_view(&TextureViewDescriptor {
                    label: Some("exa_cube_face"),
                    dimension: Some(TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..default()
                })
            })
            .collect();
    }
    prepared.pending = !jobs.faces.is_empty();
    if jobs.draws.is_empty() {
        return;
    }
    prepared.buffer.clear();
    for d in &jobs.draws {
        prepared.buffer.push(d.gpu);
    }
    prepared.buffer.write_buffer(&device, &queue);
    let Some(binding) = prepared.buffer.binding() else {
        return;
    };
    prepared.draws = Some(device.create_bind_group(
        "exa_cube_draws",
        &cache.get_bind_group_layout(&pipeline.draws),
        &BindGroupEntries::single(binding),
    ));
    let mut pipelines = Vec::with_capacity(jobs.draws.len());
    for d in &jobs.draws {
        let id = meshes.get(d.mesh).and_then(|mesh| {
            let key = CubePipelineKey {
                alpha_cut: d.cut.is_some(),
                two_sided: d.two_sided,
            };
            specialized
                .specialize(&cache, &pipeline, key, &mesh.layout)
                .ok()
        });
        pipelines.push(id);
        if let Some(cut) = d.cut
            && !prepared.cuts.contains_key(&cut)
            && let Some(image) = images.get(cut)
        {
            let group = device.create_bind_group(
                "exa_cube_cut",
                &cache.get_bind_group_layout(&pipeline.cut),
                &BindGroupEntries::sequential((&image.texture_view, &image.sampler)),
            );
            prepared.cuts.insert(cut, group);
        }
    }
    prepared.pipelines = pipelines;
}

#[allow(clippy::too_many_arguments)]
fn render_faces(
    view: ViewQuery<Has<DisplayCamera>>,
    jobs: Res<CubeJobs>,
    prepared: Option<ResMut<PreparedCubes>>,
    cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    mut ctx: RenderContext,
) {
    // Once per frame, by the scene view (not a supersampling display).
    if view.into_inner() {
        return;
    }
    let Some(mut prepared) = prepared else { return };
    if !prepared.pending {
        return;
    }
    prepared.pending = false;
    let Some(draws) = prepared.draws.clone() else {
        return;
    };
    let mut failed = Vec::new();
    for job in &jobs.faces {
        let Some(view) = prepared.layers.get(job.layer as usize) else {
            continue;
        };
        let mut complete = true;
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("exa_cube_shadow"),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &draws, &[]);
        for i in job.draws.clone() {
            let d = &jobs.draws[i as usize];
            let ready = prepared
                .pipelines
                .get(i as usize)
                .copied()
                .flatten()
                .and_then(|id| {
                    Some((
                        cache.get_render_pipeline(id)?,
                        meshes.get(d.mesh)?,
                        allocator.mesh_vertex_slice(&d.mesh)?,
                    ))
                });
            let Some((render_pipeline, mesh, vertices)) = ready else {
                complete = false;
                continue;
            };
            if let Some(cut) = d.cut {
                let Some(group) = prepared.cuts.get(&cut) else {
                    complete = false;
                    continue;
                };
                pass.set_bind_group(1, group, &[]);
            }
            pass.set_render_pipeline(render_pipeline);
            pass.set_vertex_buffer(0, vertices.buffer.slice(..));
            match &mesh.buffer_info {
                RenderMeshBufferInfo::Indexed {
                    count,
                    index_format,
                } => {
                    let Some(indices) = allocator.mesh_index_slice(&d.mesh) else {
                        complete = false;
                        continue;
                    };
                    pass.set_index_buffer(indices.buffer.slice(..), *index_format);
                    pass.draw_indexed(
                        indices.range.start..indices.range.start + count,
                        vertices.range.start as i32,
                        i..i + 1,
                    );
                }
                RenderMeshBufferInfo::NonIndexed => {
                    pass.draw(
                        vertices.range.start..vertices.range.start + mesh.vertex_count,
                        i..i + 1,
                    );
                }
            }
        }
        drop(pass);
        if !complete {
            failed.push((job.light, 1u8 << job.face));
        }
    }
    if !failed.is_empty()
        && let Ok(mut f) = jobs.failed.0.lock()
    {
        f.extend(failed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caster_scales_follow_bias_levels() {
        assert_eq!(caster_scale(0), 1.0);
        assert!((caster_scale(3) - 0.975).abs() < 1e-6);
        assert!((caster_scale(1) - 0.985).abs() < 1e-6);
    }

    #[test]
    fn face_boxes_reach_the_radius() {
        let b = face_box(Vec3::ZERO, 2.0, 0);
        // +X face: from the light out to 2 m along +X, ±2 m across.
        assert!((b.max().x - 2.0).abs() < 1e-5);
        assert!(b.min().x.abs() < 1e-5);
        assert!((b.max().y - 2.0).abs() < 1e-5);
    }
}
