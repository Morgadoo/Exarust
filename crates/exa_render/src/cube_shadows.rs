//! Cached cube shadow maps, as the original keeps its shadow maps. A light's
//! cube face is rendered only when it is needed (it looks into the camera's
//! view) and dirty: the light moved or just took its slot (every face), or a
//! caster linked to it moved (the faces its bounding sphere touched before
//! and touches now). Afterwards it stays as it is, frame after frame: a
//! still dungeon renders almost no cube faces, and a character dirties only
//! the faces it stands in. Torch flicker changes a light's colour, not its
//! position, and never dirties it. A face draws only the casters whose
//! sphere touches it.
//!
//! The faces hold the original's linear depth, distance / (R·k_c), R the
//! light's radius and k_c the caster's depth scale from its shadow-bias
//! level. They live in a pool of cube textures shared by every light; a
//! light that needs a slot takes a free one or one whose light needs none
//! this frame. Characters cast here too, skinned by their joints in the
//! caster shader. Rules: `docs/RENDERING.md` (Cached cube shadows).

use std::ops::Range;
use std::sync::{Arc, Mutex};

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path, uuid_handle};
use bevy::camera::primitives::{Aabb, CUBE_MAP_FACES, Frustum};
use bevy::camera::visibility::VisibilitySystems;
use bevy::core_pipeline::Core3dSystems;
use bevy::core_pipeline::schedule::Core3d;
use bevy::ecs::entity::EntityHashMap;
use bevy::image::{ImageCompareFunction, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::Affine3A;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Mesh, MeshVertexBufferLayoutRef, VertexAttributeDescriptor, VertexBufferLayout};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
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
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, TextureViewId,
    VertexState,
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
            .init_resource::<CubeStats>()
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
            .init_resource::<PreparedCubes>()
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

/// This frame's cube work, for the frame profiler: faces rendered and the
/// caster draws in them.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CubeStats {
    pub faces: usize,
    pub draws: usize,
}

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
    /// Marks every face out of date (the light moved or took a slot).
    pub fn invalidate(&mut self) {
        self.dirty = ALL_FACES;
    }

    /// Marks `faces` out of date (a caster in them moved, came or went).
    pub fn mark(&mut self, faces: u8) {
        self.dirty |= faces & ALL_FACES;
    }
}

/// The meshes casting into a light's cached cube, kept by the world's shadow
/// links, each with its bounding sphere and the faces that sphere touches
/// from the light ([`sphere_faces`]). The faces are worked out again when
/// the caster moves and, for all of a light's casters, when the light moves,
/// as the original refreshes a moved light's links.
#[derive(Component, Clone, Debug, Default)]
pub struct CubeCasters(pub EntityHashMap<CasterFaces>);

/// A caster's sphere (world, metres) and the faces it touches from its light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CasterFaces {
    pub faces: u8,
    pub centre: Vec3,
    pub radius: f32,
}

impl CasterFaces {
    /// The faces a sphere at `centre` of `radius` touches from `light`.
    pub fn new(light: Vec3, centre: Vec3, radius: f32) -> Self {
        Self {
            faces: sphere_faces(light, centre, radius),
            centre,
            radius,
        }
    }
}

/// The unit diagonals of the cube's side planes, in the original's order.
const SIDE_DIAGONALS: [Vec3; 6] = {
    let s = std::f32::consts::FRAC_1_SQRT_2;
    [
        Vec3::new(s, s, 0.0),
        Vec3::new(s, -s, 0.0),
        Vec3::new(s, 0.0, s),
        Vec3::new(s, 0.0, -s),
        Vec3::new(0.0, s, s),
        Vec3::new(0.0, s, -s),
    ]
};

/// Each face of the original's mask, in its bit order (+X, −X, −Y, +Y, −Z,
/// +Z): its axis and its four side planes as (diagonal, sign), the plane's
/// inward normal being `sign × diagonal`, in the original's table order.
const MASK_FACES: [(Vec3, [(usize, f32); 4]); 6] = [
    (Vec3::X, [(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)]),
    (Vec3::NEG_X, [(0, -1.0), (1, -1.0), (2, -1.0), (3, -1.0)]),
    (Vec3::NEG_Y, [(0, -1.0), (1, 1.0), (4, -1.0), (5, -1.0)]),
    (Vec3::Y, [(0, 1.0), (1, -1.0), (4, 1.0), (5, 1.0)]),
    (Vec3::NEG_Z, [(2, -1.0), (3, 1.0), (4, -1.0), (5, 1.0)]),
    (Vec3::Z, [(2, 1.0), (3, -1.0), (4, 1.0), (5, -1.0)]),
];

/// The cube faces (bit f for Bevy's face f) that a sphere at `centre` of
/// `radius` reaches from a light at `light`, as the original masks a node's
/// link: each face is a 90° frustum from the light bounded by four side
/// planes through the light at 45° between its axis and its neighbours',
/// and the sphere leaves a face out only when it lies wholly beyond one of
/// them (farther than its radius; exactly its radius still touches). The
/// side-plane distances are the sphere centre's dot product with each
/// diagonal less the light's, as the original takes them; the original's
/// face bits are then put in Bevy's face order by axis.
pub fn sphere_faces(light: Vec3, centre: Vec3, radius: f32) -> u8 {
    let distance = SIDE_DIAGONALS.map(|d| centre.dot(d) - light.dot(d));
    let mut faces = 0;
    for (axis, planes) in MASK_FACES {
        let apart = planes.iter().any(|&(i, sign)| -sign * distance[i] > radius);
        if apart {
            continue;
        }
        if let Some(f) = CUBE_MAP_FACES.iter().position(|face| face.target == axis) {
            faces |= 1 << f;
        }
    }
    faces
}

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
    /// The skinned casters' joint matrices (joint × inverse bind pose).
    joints: Vec<[f32; 16]>,
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
    /// Skinned by its joints ([`GpuDraw::joints`]).
    skinned: bool,
    /// The texture whose alpha cuts the shadow ([`shadow_cut_texture`]).
    cut: Option<AssetId<Image>>,
}

/// One draw as the caster shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GpuDraw {
    world_from_local: [f32; 16],
    clip_from_world: [f32; 16],
    light: [f32; 4],
    /// x: the first of a skinned caster's joint matrices in the frame's
    /// joint buffer.
    joints: [u32; 4],
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

/// Gives new lights a cube; a moved light's faces go out of date, and its
/// casters' faces are worked out again from where it is now.
fn add_cube_shadows(
    mut commands: Commands,
    new: NewLights,
    mut moved: Query<
        (&GlobalTransform, &mut CubeShadow, &mut CubeCasters),
        Changed<GlobalTransform>,
    >,
) {
    for (entity, light) in &new {
        commands.entity(entity).insert((
            CubeShadow {
                dirty: ALL_FACES,
                ..default()
            },
            CubeCasters::default(),
            CubeSlot {
                slot: None,
                radius: light.range / REACH,
            },
        ));
    }
    for (at, mut cube, mut casters) in &mut moved {
        cube.invalidate();
        let light = at.translation();
        for c in casters.0.values_mut() {
            c.faces = sphere_faces(light, c.centre, c.radius);
        }
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
    mut lights: Query<(&GlobalTransform, &CubeSlot, &CubeCasters, &mut CubeShadow)>,
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
    mut lights: Query<(Entity, &CubeCasters, &mut CubeShadow, &mut CubeSlot)>,
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

/// The texture whose alpha cuts a material's shadow, for materials cast the
/// original's alpha-tested way ([`ExaMaterial::cuts_shadow`]): the opacity
/// map, or else the diffuse map. Without either the shadow is whole (and
/// still cast from both sides).
fn shadow_cut_texture(material: &ExaMaterial) -> Option<AssetId<Image>> {
    if !material.cuts_shadow() {
        return None;
    }
    material
        .opacity_map
        .as_ref()
        .or(material.diffuse_map.as_ref())
        .map(Handle::id)
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
        Option<&'static SkinnedMesh>,
    ),
>;

/// The joint matrices of a skinned caster (each joint's world transform
/// times its inverse bind pose), appended to `joints`; their offset, or
/// `None` while the bind poses or a joint are missing.
fn skin_joints(
    skin: &SkinnedMesh,
    bindposes: &Assets<SkinnedMeshInverseBindposes>,
    joints: &Query<&GlobalTransform>,
    out: &mut Vec<[f32; 16]>,
) -> Option<u32> {
    let inverse = bindposes.get(&skin.inverse_bindposes)?;
    let offset = out.len();
    for (joint, bind) in skin.joints.iter().zip(inverse.iter()) {
        let Ok(at) = joints.get(*joint) else {
            out.truncate(offset);
            return None;
        };
        out.push((at.to_matrix() * *bind).to_cols_array());
    }
    Some(offset as u32)
}

/// Lists the casters of every face both needed and dirty, for the render
/// world, and marks those faces done. A face draws the casters whose sphere
/// touches it (or whose faces are not known yet); a skinned caster's joints
/// are posed once for all the faces it is drawn into.
#[allow(clippy::too_many_arguments)]
fn build_jobs(
    mut jobs: ResMut<CubeJobs>,
    mut lights: Query<(
        Entity,
        &GlobalTransform,
        &CubeSlot,
        &CubeCasters,
        &mut CubeShadow,
    )>,
    casters: CasterData,
    joints: Query<&GlobalTransform>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    materials: Res<Assets<ExaMaterial>>,
    mut posed: Local<EntityHashMap<Option<u32>>>,
    mut stats: ResMut<CubeStats>,
) {
    jobs.faces.clear();
    jobs.draws.clear();
    jobs.joints.clear();
    posed.clear();
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
            for (&caster, c) in &list.0 {
                if c.faces & (1 << face) == 0 {
                    continue;
                }
                let Ok((mesh, at, bias, material, skin)) = casters.get(caster) else {
                    continue;
                };
                let joint_offset = match skin {
                    Some(skin) => {
                        let offset = *posed.entry(caster).or_insert_with(|| {
                            skin_joints(skin, &bindposes, &joints, &mut jobs.joints)
                        });
                        // Not posable yet: left out rather than drawn at rest.
                        let Some(offset) = offset else { continue };
                        Some(offset)
                    }
                    None => None,
                };
                let k_c = caster_scale(bias.map_or(0, |b| b.0));
                let material = material.and_then(|m| materials.get(&m.0));
                let cut = material.and_then(shadow_cut_texture);
                jobs.draws.push(CasterDraw {
                    mesh: mesh.0.id(),
                    gpu: GpuDraw {
                        world_from_local: at.to_matrix().to_cols_array(),
                        clip_from_world: clip_from_world.to_cols_array(),
                        light: position.extend(1.0 / (slot.radius * k_c)).to_array(),
                        joints: [joint_offset.unwrap_or(0), 0, 0, 0],
                    },
                    // The original turns culling off for its alpha-tested
                    // casters, with or without a texture to cut with.
                    two_sided: material
                        .is_some_and(|m| m.cuts_shadow() || m.params.flags & flags::TWO_SIDED != 0),
                    skinned: joint_offset.is_some(),
                    cut,
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
    *stats = CubeStats {
        faces: jobs.faces.len(),
        draws: jobs.draws.len(),
    };
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
    skinned: bool,
}

impl CubePipeline {
    fn vertex_attributes(key: CubePipelineKey) -> Vec<VertexAttributeDescriptor> {
        let mut attributes = vec![Mesh::ATTRIBUTE_POSITION.at_shader_location(0)];
        if key.alpha_cut {
            attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(1));
        }
        if key.skinned {
            attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(2));
            attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(3));
        }
        attributes
    }
}

impl SpecializedMeshPipeline for CubePipeline {
    type Key = CubePipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let attributes = Self::vertex_attributes(key);
        let mut defs = Vec::new();
        let mut layouts = vec![self.draws.clone()];
        if key.alpha_cut {
            defs.push("ALPHA_CUT".into());
            layouts.push(self.cut.clone());
        }
        if key.skinned {
            defs.push("SKINNED".into());
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
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                storage_buffer_read_only_sized(false, None),
                // The skinned casters' joint matrices.
                storage_buffer_read_only_sized(false, None),
            ),
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
    joints: RawBufferVec<[f32; 16]>,
    draws: Option<BindGroup>,
    /// The bind group of each shadow-cutting texture, with the texture view
    /// it was made for: an image replaced under its handle (a streamed
    /// texture arriving) gets a new view and a new group.
    cuts: HashMap<AssetId<Image>, (TextureViewId, BindGroup)>,
    /// Per draw, its pipeline.
    pipelines: Vec<Option<CachedRenderPipelineId>>,
    /// Specialized ids, keyed by mesh and key.
    specialized: HashMap<(VertexBufferLayout, CubePipelineKey), CachedRenderPipelineId>,
    /// Per pool layer, a view to render into.
    layers: Vec<TextureView>,
    /// Faces left for this frame (rendered once, by the first scene view).
    pending: bool,
}

impl Default for PreparedCubes {
    fn default() -> Self {
        Self {
            buffer: RawBufferVec::new(BufferUsages::STORAGE),
            joints: RawBufferVec::new(BufferUsages::STORAGE),
            draws: None,
            cuts: HashMap::default(),
            pipelines: Vec::new(),
            specialized: HashMap::default(),
            layers: Vec::new(),
            pending: false,
        }
    }
}

/// Hands every face of the frame back as not drawn, so it stays dirty: a
/// face never drawn reads as wholly shadowed.
fn fail_all(jobs: &CubeJobs) {
    if jobs.faces.is_empty() {
        return;
    }
    if let Ok(mut f) = jobs.failed.0.lock() {
        f.extend(jobs.faces.iter().map(|j| (j.light, 1u8 << j.face)));
    }
}

/// Returns the cached pipeline for `key`, or creates and stores it once.
fn cached_pipeline<K, V>(
    cache: &mut HashMap<K, V>,
    key: K,
    create: impl FnOnce() -> Option<V>,
) -> Option<V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Copy,
{
    if let Some(value) = cache.get(&key) {
        return Some(*value);
    }
    let value = create()?;
    cache.insert(key, value);
    Some(value)
}

#[allow(clippy::too_many_arguments)]
fn prepare_draws(
    jobs: Res<CubeJobs>,
    pipeline: Option<Res<CubePipeline>>,
    mut specialized: ResMut<SpecializedMeshPipelines<CubePipeline>>,
    cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut prepared: ResMut<PreparedCubes>,
) {
    let (Some(pipeline), Some(pool)) = (pipeline, images.get(CUBE_POOL.id())) else {
        prepared.pending = false;
        fail_all(&jobs);
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
    prepared.joints.clear();
    for m in &jobs.joints {
        prepared.joints.push(*m);
    }
    // The binding needs a buffer even when nothing is skinned.
    if prepared.joints.is_empty() {
        prepared.joints.push(Mat4::IDENTITY.to_cols_array());
    }
    prepared.joints.write_buffer(&device, &queue);
    let (Some(binding), Some(joints)) = (prepared.buffer.binding(), prepared.joints.binding())
    else {
        return;
    };
    prepared.draws = Some(device.create_bind_group(
        "exa_cube_draws",
        &cache.get_bind_group_layout(&pipeline.draws),
        &BindGroupEntries::sequential((binding, joints)),
    ));
    let mut pipelines = Vec::with_capacity(jobs.draws.len());
    for d in &jobs.draws {
        let id = meshes.get(d.mesh).and_then(|mesh| {
            let key = CubePipelineKey {
                alpha_cut: d.cut.is_some(),
                two_sided: d.two_sided,
                skinned: d.skinned,
            };
            let Ok(layout) = mesh
                .layout
                .0
                .get_layout(&CubePipeline::vertex_attributes(key))
            else {
                return None;
            };
            cached_pipeline(&mut prepared.specialized, (layout, key), || {
                specialized
                    .specialize(&cache, &pipeline, key, &mesh.layout)
                    .ok()
            })
        });
        pipelines.push(id);
        if let Some(cut) = d.cut
            && let Some(image) = images.get(cut)
            && prepared
                .cuts
                .get(&cut)
                .is_none_or(|(view, _)| *view != image.texture_view.id())
        {
            let group = device.create_bind_group(
                "exa_cube_cut",
                &cache.get_bind_group_layout(&pipeline.cut),
                &BindGroupEntries::sequential((&image.texture_view, &image.sampler)),
            );
            prepared.cuts.insert(cut, (image.texture_view.id(), group));
        }
    }
    prepared.pipelines = pipelines;
}

#[allow(clippy::too_many_arguments)]
fn render_faces(
    view: ViewQuery<Has<DisplayCamera>>,
    jobs: Res<CubeJobs>,
    mut prepared: ResMut<PreparedCubes>,
    cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    mut ctx: RenderContext,
) {
    // Once per frame, by the scene view (not a supersampling display).
    if view.into_inner() {
        return;
    }
    if !prepared.pending {
        return;
    }
    prepared.pending = false;
    // Faces without casters are only cleared; faces with casters need the
    // frame's draws.
    let draws = prepared.draws.clone();
    if draws.is_none() && !jobs.draws.is_empty() {
        fail_all(&jobs);
        return;
    }
    let mut failed = Vec::new();
    // A GPU timing span for the frame profiler, when render diagnostics are
    // on.
    let diagnostics = ctx.diagnostic_recorder();
    let span = diagnostics
        .as_deref()
        .map(|d| d.time_span(ctx.command_encoder(), "exa_cube_shadows"));
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
        if let Some(draws) = &draws {
            pass.set_bind_group(0, draws, &[]);
        }
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
                let Some((_, group)) = prepared.cuts.get(&cut) else {
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
    if let Some(span) = span {
        span.end(ctx.command_encoder());
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
    fn shadow_cuts_follow_the_alpha_tested_caster() {
        const OPACITY: Handle<Image> = uuid_handle!("0c7b6a52-5d0e-4c55-9a7e-6f1b2c3d4e01");
        const DIFFUSE: Handle<Image> = uuid_handle!("0c7b6a52-5d0e-4c55-9a7e-6f1b2c3d4e02");
        // A blended cobweb marked to cut: its opacity map cuts.
        let cobweb = ExaMaterial {
            opacity_map: Some(OPACITY),
            diffuse_map: Some(DIFFUSE),
            alpha_mode: AlphaMode::Blend,
            cut_shadow: true,
            ..default()
        };
        assert_eq!(shadow_cut_texture(&cobweb), Some(OPACITY.id()));
        // Blended and not marked: a whole shadow.
        let glass = ExaMaterial {
            cut_shadow: false,
            ..cobweb.clone()
        };
        assert_eq!(shadow_cut_texture(&glass), None);
        // Alpha-tested with only a diffuse map: the diffuse map cuts.
        let grate = ExaMaterial {
            diffuse_map: Some(DIFFUSE),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        };
        assert_eq!(shadow_cut_texture(&grate), Some(DIFFUSE.id()));
        // Alpha-tested without a texture: nothing to cut with.
        let bare = ExaMaterial {
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        };
        assert_eq!(shadow_cut_texture(&bare), None);
    }

    #[test]
    fn face_boxes_reach_the_radius() {
        let b = face_box(Vec3::ZERO, 2.0, 0);
        // +X face: from the light out to 2 m along +X, ±2 m across.
        assert!((b.max().x - 2.0).abs() < 1e-5);
        assert!(b.min().x.abs() < 1e-5);
        assert!((b.max().y - 2.0).abs() < 1e-5);
    }

    /// A small sphere well along one axis touches only that axis's face; a
    /// sphere on a diagonal touches the faces it straddles; a sphere around
    /// the light touches all six. Each face index is Bevy's, the one the
    /// face's view is built from.
    #[test]
    fn spheres_touch_the_faces_they_reach() {
        let light = Vec3::new(1.0, 2.0, 3.0);
        for (f, face) in CUBE_MAP_FACES.iter().enumerate() {
            assert_eq!(
                sphere_faces(light, light + face.target * 4.0, 0.3),
                1 << f,
                "face {f}"
            );
        }
        let diagonal = sphere_faces(light, light + Vec3::new(3.0, 0.0, 3.0), 0.5);
        assert_eq!(diagonal.count_ones(), 2, "{diagonal:#b}");
        assert_eq!(sphere_faces(light, light + Vec3::X * 0.2, 0.5), ALL_FACES);
        // A sphere far below spans only the downward face.
        let below = sphere_faces(light, light - Vec3::Y * 5.0, 1.0);
        assert_eq!(below.count_ones(), 1);
    }

    /// Only the side planes leave a face out: a sphere 1 m behind the light
    /// and 0.9 m wide lies within 0.9 m of every side plane (its centre is
    /// 1/√2 m from each of the +X face's), so it keeps all six faces, the
    /// +X face's too. On the +X/+Y edge with no radius it is exactly on a
    /// side plane of each and keeps both.
    #[test]
    fn only_the_side_planes_leave_a_face_out() {
        let light = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(
            sphere_faces(light, light - Vec3::X, 0.9),
            ALL_FACES,
            "behind the light"
        );
        let x = CUBE_MAP_FACES
            .iter()
            .position(|f| f.target == Vec3::X)
            .unwrap();
        let y = CUBE_MAP_FACES
            .iter()
            .position(|f| f.target == Vec3::Y)
            .unwrap();
        assert_eq!(
            sphere_faces(Vec3::ZERO, Vec3::new(10.0, 10.0, 0.0), 0.0),
            (1 << x) | (1 << y)
        );
    }

    /// A light that moves past a caster finds it in another face: the
    /// caster's faces follow the light, or the face it now lies in would
    /// leave it out and cut its shadow.
    #[test]
    fn a_moved_light_works_out_its_casters_faces_again() {
        let mut app = App::new();
        app.add_systems(Update, add_cube_shadows);
        let caster = Entity::from_raw_u32(9).unwrap();
        let centre = Vec3::new(2.0, 1.0, 0.0);
        let light = app
            .world_mut()
            .spawn((
                PointLight::default(),
                GlobalTransform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                CubeShadow::default(),
                CubeSlot {
                    slot: None,
                    radius: 8.0,
                },
                CubeCasters(
                    [(
                        caster,
                        CasterFaces::new(Vec3::new(0.0, 1.0, 0.0), centre, 0.2),
                    )]
                    .into_iter()
                    .collect(),
                ),
            ))
            .id();
        let faces = |app: &App| app.world().get::<CubeCasters>(light).unwrap().0[&caster].faces;
        let towards_x = faces(&app);
        assert_eq!(towards_x.count_ones(), 1);
        // The light moves to the caster's other side.
        app.world_mut()
            .entity_mut(light)
            .insert(GlobalTransform::from_translation(Vec3::new(4.0, 1.0, 0.0)));
        app.update();
        let after = faces(&app);
        assert_eq!(after.count_ones(), 1);
        assert_ne!(after, towards_x);
        assert_eq!(after, sphere_faces(Vec3::new(4.0, 1.0, 0.0), centre, 0.2));
        assert_eq!(
            app.world().get::<CubeShadow>(light).unwrap().dirty,
            ALL_FACES
        );
    }

    #[test]
    fn skinned_casters_read_their_joints() {
        let key = CubePipelineKey {
            alpha_cut: false,
            two_sided: false,
            skinned: true,
        };
        let ids: Vec<_> = CubePipeline::vertex_attributes(key)
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(
            ids,
            [
                Mesh::ATTRIBUTE_POSITION.id,
                Mesh::ATTRIBUTE_JOINT_INDEX.id,
                Mesh::ATTRIBUTE_JOINT_WEIGHT.id
            ]
        );
    }

    #[test]
    fn marks_add_to_the_dirty_faces() {
        let mut cube = CubeShadow::default();
        cube.mark(0b10);
        cube.mark(0b100);
        assert_eq!(cube.dirty, 0b110);
        cube.mark(u8::MAX);
        assert_eq!(cube.dirty, ALL_FACES);
    }

    #[test]
    fn the_pipeline_cache_specializes_once() {
        let mut cache = HashMap::default();
        let mut calls = 0;
        let layout = VertexBufferLayout::default();
        let plain = CubePipelineKey {
            alpha_cut: false,
            two_sided: false,
            skinned: false,
        };
        let first = cached_pipeline(&mut cache, (layout.clone(), plain), || {
            calls += 1;
            Some(CachedRenderPipelineId::new(1))
        });
        assert_eq!(first, Some(CachedRenderPipelineId::new(1)));
        assert_eq!(calls, 1);

        let second = cached_pipeline(&mut cache, (layout.clone(), plain), || {
            calls += 1;
            Some(CachedRenderPipelineId::new(2))
        });
        assert_eq!(second, Some(CachedRenderPipelineId::new(1)));
        assert_eq!(calls, 1);

        let cut = CubePipelineKey {
            alpha_cut: true,
            two_sided: false,
            skinned: false,
        };
        let failed = cached_pipeline(&mut cache, (layout, cut), || {
            calls += 1;
            None
        });
        assert_eq!(failed, None);
        assert_eq!(calls, 2);
    }
}
