//! The original's voxel global illumination: a block of 20 cm voxels where
//! light from the level's lights spreads through the open cells, one step
//! per frame, and surfaces take their ambient from it. The occupancy comes
//! from the world (`exa_world::gi_volume`); this module owns the volume
//! textures, the propagation pass (`gi.wesl`) and the lights it injects:
//! point lights, sector by sector as the original draws them from its cell
//! lists, and voxel lights (glowing items), which add their light straight
//! into the volume around themselves.
//! Rules: `docs/RENDERING.md` (Voxel GI).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::core_pipeline::Core3dSystems;
use bevy::core_pipeline::schedule::Core3d;
use bevy::platform::collections::HashMap;
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
            .init_resource::<GiSectorLights>()
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
    /// The faint light every open voxel gains per step, per channel (level
    /// record 0x30, as stored, × 0.001).
    pub base_ambient: Vec3,
    /// The block's first 6 m cell in the level's sector grid.
    pub sector: IVec2,
    /// Steps run at once when the volume is created.
    pub warm_up: u32,
}

/// The GI textures and where they sit.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiTextures {
    pub occupancy: Handle<Image>,
    pub light: Handle<Image>,
    scratch: Handle<Image>,
    pub origin: Vec3,
    pub extent: Vec3,
    /// Voxel edge, metres, and the texture's size (X, Z, Y).
    voxel: f32,
    dims: UVec3,
    /// The per-column floors the block was built with (X fastest, then Z).
    pub floors: Arc<Vec<u8>>,
    /// The block's first 6 m cell in the level's sector grid.
    pub sector: IVec2,
    base_ambient: Vec3,
    /// Steps run at once when the volume is created.
    warm_up: u32,
    /// Which volume these are (counting from 1), so the render world starts
    /// each volume's warm-up once.
    serial: u32,
}

impl GiTextures {
    /// The lowest slice sector (`x`, `z`) of the block updates; 255 when it
    /// has no geometry.
    pub fn sector_floor(&self, x: usize, z: usize) -> u8 {
        let (w, h) = (self.dims.x as usize, self.dims.y as usize);
        let (cx, cz) = (
            MARGIN as usize + SECTOR_VOXELS as usize * x,
            MARGIN as usize + SECTOR_VOXELS as usize * z,
        );
        if x >= SECTORS
            || z >= SECTORS
            || cx + SECTOR_VOXELS as usize > w
            || cz + SECTOR_VOXELS as usize > h
        {
            return u8::MAX;
        }
        self.floors.get(cx + cz * w).copied().unwrap_or(u8::MAX)
    }

    /// Voxels along X, Z and Y.
    pub fn dims(&self) -> UVec3 {
        self.dims
    }
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

/// The lights one sector of a block records: the lights of its 6 m cell in
/// the order the cell's flood lists them, at most twelve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GiSector {
    lights: [u32; MAX_SECTOR_LIGHTS],
    count: u8,
}

impl GiSector {
    /// From light ids (the shaders' record ids), keeping the first
    /// [`MAX_SECTOR_LIGHTS`].
    pub fn new(ids: impl IntoIterator<Item = u32>) -> Self {
        let mut out = Self::default();
        for id in ids {
            if (out.count as usize) < MAX_SECTOR_LIGHTS {
                out.lights[out.count as usize] = id;
                out.count += 1;
            }
        }
        out
    }

    /// The sector's light ids, in the cell's order.
    pub fn ids(&self) -> &[u32] {
        &self.lights[..self.count as usize]
    }
}

/// The manager's per-frame state for the block's 8 × 8 sectors: whether the
/// camera marks a sector for update (the reach and frustum test, widened by
/// one sector), and the lights its 6 m cell records. The world fills it
/// every frame (`exa_world::gi_volume`), as the original marks sectors and
/// reads each block's lights from the cell lists.
#[derive(Resource, Clone, Default)]
pub struct GiSectorLights(pub Arc<GiSectors>);

/// [`GiSectorLights`]'s body.
#[derive(Clone, Debug)]
pub struct GiSectors {
    /// Sectors drawn this frame, X fastest (texture X, then Z).
    pub drawn: [bool; SECTORS * SECTORS],
    /// Each sector's lights.
    pub sectors: [GiSector; SECTORS * SECTORS],
}

impl Default for GiSectors {
    fn default() -> Self {
        Self {
            drawn: [false; SECTORS * SECTORS],
            sectors: [GiSector::default(); SECTORS * SECTORS],
        }
    }
}

impl GiSectors {
    /// The state of sector (`x`, `z`), both 0..8.
    pub fn sector(&self, x: usize, z: usize) -> &GiSector {
        &self.sectors[x + z * SECTORS]
    }
}

/// A sector as the light grouping reads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct GiSectorPlan {
    /// The manager marked the sector for update.
    pub drawn: bool,
    /// The lowest slice it updates.
    pub floor: u8,
    /// The lights its cell records.
    pub lights: GiSector,
}

/// A group of sectors the original draws together: the same light set and
/// floors within two slices, merged while the group keeps at most twelve
/// lights.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GiGroupPlan {
    /// The lowest slice any sector of the group updates.
    pub floor: u8,
    /// The group's lights, in the order its sectors list them.
    pub lights: Vec<u32>,
}

/// The original's sector grouping: sectors are joined into a group while
/// their light lists match exactly in the same order and their floor slices
/// differ by less than 3. Two groups then merge, in two passes, when at
/// least one light is common (or both are empty) and their union stays
/// within twelve lights. Returns the groups in creation order and each
/// sector's group, or 255 for a sector not drawn.
pub fn group_sectors(
    sectors: &[GiSectorPlan; SECTORS * SECTORS],
) -> (Vec<GiGroupPlan>, [u8; SECTORS * SECTORS]) {
    let mut groups: Vec<GiGroupPlan> = Vec::new();
    let mut of_sector = [u8::MAX; SECTORS * SECTORS];
    for (i, sector) in sectors.iter().enumerate() {
        // A sector with no geometry (floor 255) is never drawn, and neither
        // is one the camera did not mark.
        if !sector.drawn || sector.floor == u8::MAX {
            continue;
        }
        let ids = sector.lights.ids();
        let joined = groups.iter().position(|g| {
            g.lights == ids && (i32::from(g.floor) - i32::from(sector.floor)).abs() < 3
        });
        match joined {
            Some(g) => {
                groups[g].floor = groups[g].floor.min(sector.floor);
                of_sector[i] = g as u8;
            }
            None => {
                groups.push(GiGroupPlan {
                    floor: sector.floor,
                    lights: ids.to_vec(),
                });
                of_sector[i] = (groups.len() - 1) as u8;
            }
        }
    }
    // Two passes over the pairs, as the original's merge loop runs.
    for _ in 0..2 {
        let mut i = 0;
        while i < groups.len() {
            let mut j = i + 1;
            while j < groups.len() {
                let common = groups[j]
                    .lights
                    .iter()
                    .filter(|l| groups[i].lights.contains(l))
                    .count();
                let union = groups[i].lights.len() + groups[j].lights.len() - common;
                let disjoint =
                    common == 0 && (!groups[i].lights.is_empty() || !groups[j].lights.is_empty());
                if disjoint || union > MAX_SECTOR_LIGHTS {
                    j += 1;
                    continue;
                }
                // The second group's lights not already in the first join
                // it, then its floor, and it disappears.
                let added: Vec<u32> = groups[j]
                    .lights
                    .iter()
                    .copied()
                    .filter(|l| !groups[i].lights.contains(l))
                    .collect();
                groups[i].lights.extend(added);
                groups[i].floor = groups[i].floor.min(groups[j].floor);
                groups.remove(j);
                for g in &mut of_sector {
                    if *g == j as u8 {
                        *g = i as u8;
                    } else if *g != u8::MAX && *g as usize > j {
                        *g -= 1;
                    }
                }
            }
            i += 1;
        }
    }
    (groups, of_sector)
}

/// The lights injected into the volume this frame.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiLights {
    /// The groups' lights, one run of [`GiGroupUniform::count`] after
    /// another.
    lights: Vec<GiLight>,
    groups: Vec<GiGroupUniform>,
    /// Per sector (X fastest), its group or 255.
    sector_groups: [u8; SECTORS * SECTORS],
    voxel_lights: Vec<GiVoxelLight>,
}

impl Default for GiLights {
    fn default() -> Self {
        Self {
            lights: Vec::new(),
            groups: Vec::new(),
            sector_groups: [u8::MAX; SECTORS * SECTORS],
            voxel_lights: Vec::new(),
        }
    }
}

/// A group of sectors: the slice it draws from, and its run of lights.
#[derive(Clone, Copy, Default, ShaderType)]
struct GiGroupUniform {
    floor: u32,
    first: u32,
    count: u32,
    _pad: u32,
}

#[derive(Clone, Copy, Default, ShaderType)]
struct GiLight {
    position_radius: Vec4,
    colour: Vec4,
    /// Vertical shaping (y, z), as the surface shader reads it.
    shaping: Vec4,
}

/// Sectors across a block (X and Z): the original's 8 × 8 of 6 m cells.
pub const SECTORS: usize = 8;
/// Voxels along one side of a sector's interior, 6 m at 20 cm.
const SECTOR_VOXELS: u32 = 30;
/// Voxels of margin on each side of the sectors.
const MARGIN: u32 = 8;
/// The most lights one sector records and one group injects.
pub const MAX_SECTOR_LIGHTS: usize = 12;
/// Groups of sectors a block draws at once, one per sector at most.
const MAX_GROUPS: usize = SECTORS * SECTORS;
/// The flat light array the groups index into.
const MAX_GROUP_LIGHTS: usize = MAX_GROUPS * MAX_SECTOR_LIGHTS;

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
    /// The base ambient every open voxel gains per step, per channel.
    base: Vec4,
    /// The groups of sectors drawn this frame.
    groups: [GiGroupUniform; MAX_GROUPS],
    /// Per sector (X fastest), its group or u32::MAX.
    sector_groups: [UVec4; SECTORS * SECTORS / 4],
    lights: [GiLight; MAX_GROUP_LIGHTS],
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
    let corner = IVec3::new(
        at.x.round_ties_even() as i32,
        at.y.round_ties_even() as i32,
        at.z.round_ties_even() as i32,
    );
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
    let empty = vec![0u8; voxels * 4];
    let light = images.add(light_image(size, empty.clone()));
    let scratch = images.add(volume_image(size, TextureFormat::Rgba8Unorm, empty, true));
    let extent = Vec3::new(size.x as f32, size.z as f32, size.y as f32) * volume.voxel;
    warm_up.serial += 1;
    warm_up.steps = volume.warm_up;
    let textures = GiTextures {
        occupancy,
        light,
        scratch,
        origin: volume.origin,
        extent,
        voxel: volume.voxel,
        dims: size,
        floors: volume.floors.clone(),
        sector: volume.sector,
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

/// A light as the grouping and the uniforms need it.
struct LightSource {
    colour: Vec3,
    intensity: f32,
    /// The radius the injection falls off over, metres.
    radius: f32,
    position: Vec3,
    multiplier: f32,
    shaping: Vec2,
}

/// The point lights the injection reads.
type GiLightQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static PointLight,
        &'static GlobalTransform,
        &'static crate::lights::LightId,
        Option<&'static GiMultiplier>,
        Option<&'static crate::lights::LightShaping>,
    ),
>;

#[allow(clippy::too_many_arguments)]
fn gather_lights(
    mut gi: ResMut<GiLights>,
    lights: GiLightQuery,
    voxel_lights: Query<(&VoxelLight, &GlobalTransform)>,
    textures: Option<Res<GiTextures>>,
    pending: Option<Res<GiPendingTextures>>,
    sectors: Option<Res<GiSectorLights>>,
) {
    gi.lights.clear();
    gi.groups.clear();
    gi.voxel_lights.clear();
    gi.sector_groups = [u8::MAX; SECTORS * SECTORS];
    let textures = pending.as_ref().map(|p| &p.0).or(textures.as_deref());
    let (Some(textures), Some(sectors)) = (textures, sectors) else {
        return;
    };
    gather_voxel_lights(&mut gi.voxel_lights, textures, voxel_lights.iter());
    // The lights the cells record, by their shader id.
    let mut sources: HashMap<u32, LightSource> = HashMap::default();
    for (light, at, id, share, shaping) in &lights {
        let c = light.color.to_linear();
        sources.insert(
            id.0,
            LightSource {
                colour: Vec3::new(c.red, c.green, c.blue),
                intensity: light.intensity / LUMENS_PER_INTENSITY,
                radius: light.range / REACH,
                position: at.translation(),
                multiplier: share.map_or(1.0, |m| m.0),
                shaping: shaping.map_or(Vec2::ONE, |s| s.0),
            },
        );
    }
    // Each drawn sector keeps the lights of its cell that the downward
    // shaping lets reach its floor slice, in the cell's order.
    let mut plans = [GiSectorPlan::default(); SECTORS * SECTORS];
    for z in 0..SECTORS {
        for x in 0..SECTORS {
            let i = x + z * SECTORS;
            let floor = textures.sector_floor(x, z);
            let drawn = sectors.0.drawn[i] && floor != u8::MAX;
            let mut kept = [0u32; MAX_SECTOR_LIGHTS];
            let mut count = 0;
            if drawn {
                let centre = textures.origin
                    + Vec3::new(
                        4.6 + 6.0 * x as f32,
                        (f32::from(floor) + 0.5) * textures.voxel,
                        4.6 + 6.0 * z as f32,
                    );
                for &id in sectors.0.sector(x, z).ids() {
                    let Some(source) = sources.get(&id) else {
                        continue;
                    };
                    if !shape_reaches(source, centre) {
                        continue;
                    }
                    kept[count] = id;
                    count += 1;
                }
            }
            plans[i] = GiSectorPlan {
                drawn,
                floor,
                lights: GiSector::new(kept[..count].iter().copied()),
            };
        }
    }
    let (groups, of_sector) = group_sectors(&plans);
    for group in &groups {
        let first = gi.lights.len();
        for id in &group.lights {
            let Some(source) = sources.get(id) else {
                continue;
            };
            let intensity = source.intensity * source.multiplier;
            gi.lights.push(GiLight {
                position_radius: source.position.extend(source.radius),
                colour: (source.colour * intensity * INJECTION).extend(0.0),
                shaping: source.shaping.extend(0.0).extend(0.0),
            });
        }
        let count = (gi.lights.len() - first) as u32;
        gi.groups.push(GiGroupUniform {
            floor: u32::from(group.floor),
            first: first as u32,
            count,
            _pad: 0,
        });
    }
    gi.sector_groups = of_sector;
}

/// The original's downward-shaping test at a sector's floor slice: the light
/// must still carry vertically at the sector centre. Lights that are not
/// shaped (y = 1) always pass. The horizontal offset is folded one 6 m cell
/// towards zero, so a light within half a cell of the centre counts as
/// straight above it.
fn shape_reaches(source: &LightSource, centre: Vec3) -> bool {
    let y = source.shaping.x;
    if y == 1.0 {
        return true;
    }
    let fold = |d: f32| {
        if d.abs() <= 3.0 {
            0.0
        } else {
            d - d.signum() * 3.0
        }
    };
    let (dx, dz) = (
        fold(source.position.x - centre.x),
        fold(source.position.z - centre.z),
    );
    let dy = source.position.y - centre.y;
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    let c = if distance > 0.0 { dy / distance } else { 0.0 };
    c + (1.0 - c) * y > 1.19e-7
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
    let (Some(light), Some(occupancy), Some(scratch)) = (
        images.get(&textures.light),
        images.get(&textures.occupancy),
        images.get(&textures.scratch),
    ) else {
        return;
    };
    let mut entries = [GiLight::default(); MAX_GROUP_LIGHTS];
    for (slot, l) in entries.iter_mut().zip(&lights.lights) {
        *slot = *l;
    }
    let mut groups = [GiGroupUniform::default(); MAX_GROUPS];
    for (slot, g) in groups.iter_mut().zip(&lights.groups) {
        *slot = *g;
    }
    let mut sector_groups = [UVec4::splat(u32::MAX); SECTORS * SECTORS / 4];
    for (i, &group) in lights.sector_groups.iter().enumerate() {
        sector_groups[i / 4][i % 4] = if group == u8::MAX {
            u32::MAX
        } else {
            u32::from(group)
        };
    }
    let mut voxel_entries = [GiVoxelLight::default(); MAX_VOXEL_LIGHTS];
    for (slot, l) in voxel_entries.iter_mut().zip(&lights.voxel_lights) {
        *slot = *l;
    }
    let mut params = UniformBuffer::from(GiParams {
        origin: textures.origin.extend(0.0),
        extent: textures.extent.extend(0.0),
        base: textures.base_ambient.extend(0.0),
        groups,
        sector_groups,
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
            light: Handle::default(),
            scratch: Handle::default(),
            origin: Vec3::ZERO,
            extent: Vec3::ONE,
            voxel: 0.2,
            dims: UVec3::ONE,
            floors: Arc::new(vec![
                0;
                SECTORS
                    * SECTOR_VOXELS as usize
                    * SECTORS
                    * SECTOR_VOXELS as usize
            ]),
            sector: IVec2::ZERO,
            base_ambient: Vec3::ZERO,
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
            base_ambient: Vec3::ZERO,
            sector: IVec2::ZERO,
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
        assert_eq!((lo, hi), (UVec3::new(97, 97, 27), UVec3::new(103, 103, 34)));
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

    fn plan(drawn: bool, floor: u8, lights: &[u32]) -> GiSectorPlan {
        GiSectorPlan {
            drawn,
            floor,
            lights: GiSector::new(lights.iter().copied()),
        }
    }

    #[test]
    fn sectors_with_the_same_lights_share_a_group_and_its_lowest_floor() {
        let mut sectors = [GiSectorPlan::default(); 64];
        sectors[0] = plan(true, 46, &[3, 7]);
        sectors[1] = plan(true, 48, &[3, 7]);
        sectors[2] = plan(true, 49, &[3, 7]);
        // An unmarked or empty sector is left out.
        sectors[3] = plan(false, 46, &[3, 7]);
        sectors[4] = plan(true, u8::MAX, &[3, 7]);
        let (groups, of_sector) = group_sectors(&sectors);
        // The sector pass keeps floors within two slices apart, and the
        // group pass then joins the equal light sets whatever their floors.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights, vec![3, 7]);
        assert_eq!(groups[0].floor, 46);
        assert_eq!(&of_sector[..3], &[0, 0, 0]);
        assert_eq!(of_sector[3], u8::MAX);
        assert_eq!(of_sector[4], u8::MAX);
    }

    #[test]
    fn far_floors_alone_do_not_split_equal_light_sets() {
        let mut sectors = [GiSectorPlan::default(); 64];
        sectors[0] = plan(true, 10, &[1]);
        sectors[1] = plan(true, 100, &[1]);
        let (groups, _) = group_sectors(&sectors);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].floor, 10);
    }

    #[test]
    fn groups_merge_only_where_their_lights_overlap_and_stay_within_twelve() {
        let mut sectors = [GiSectorPlan::default(); 64];
        // Two groups with different sets: [1, 2] and [2, 5], far enough
        // apart in floor to start apart. The merge pass joins them (they
        // share 2) and takes the first group's floor.
        sectors[0] = plan(true, 10, &[1, 2]);
        sectors[1] = plan(true, 20, &[2, 5]);
        let (groups, of_sector) = group_sectors(&sectors);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights, vec![1, 2, 5]);
        assert_eq!(groups[0].floor, 10);
        assert_eq!(&of_sector[..2], &[0, 0]);

        // Disjoint light sets never merge, even when both are empty.
        let mut sectors = [GiSectorPlan::default(); 64];
        sectors[0] = plan(true, 10, &[1]);
        sectors[1] = plan(true, 10, &[2]);
        let (groups, _) = group_sectors(&sectors);
        assert_eq!(groups.len(), 2);

        // Eight lights and six sharing one make thirteen, over the cap.
        let eight: Vec<u32> = (0..8).collect();
        let six: Vec<u32> = (7..13).collect();
        sectors[0] = plan(true, 10, &eight);
        sectors[1] = plan(true, 10, &six);
        let (groups, _) = group_sectors(&sectors);
        assert_eq!(groups.len(), 2);

        // Five sharing one with eight gives twelve and merges.
        let five: Vec<u32> = (7..12).collect();
        sectors[1] = plan(true, 10, &five);
        let (groups, _) = group_sectors(&sectors);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights.len(), 12);
    }

    #[test]
    fn shaped_lights_must_still_reach_a_sectors_floor_slice() {
        let source = |position: Vec3, y: f32| LightSource {
            colour: Vec3::ONE,
            intensity: 1.0,
            radius: 10.0,
            position,
            multiplier: 1.0,
            shaping: Vec2::new(y, 1.0),
        };
        let centre = Vec3::new(0.0, 1.0, 0.0);
        // Straight above passes, and so does a light within half a cell.
        assert!(shape_reaches(
            &source(Vec3::new(0.0, 5.0, 0.0), 0.0),
            centre
        ));
        assert!(shape_reaches(
            &source(Vec3::new(2.9, 5.0, 0.0), 0.0),
            centre
        ));
        // At its own height, or below the slice, a shaped light is cut.
        assert!(!shape_reaches(
            &source(Vec3::new(0.0, 1.0, 0.0), 0.0),
            centre
        ));
        assert!(!shape_reaches(
            &source(Vec3::new(0.0, -3.0, 0.0), 0.0),
            centre
        ));
        // Its horizontal offset folds one cell towards zero: 2 m after the
        // fold of a 5 m offset, below a 5 m rise, still carries.
        assert!(shape_reaches(
            &source(Vec3::new(5.0, 6.0, 0.0), 0.0),
            centre
        ));
        // An unshaped light always passes.
        assert!(shape_reaches(
            &source(Vec3::new(5.0, -20.0, 0.0), 1.0),
            centre
        ));
    }

    #[test]
    fn voxel_light_corners_round_to_even() {
        // The corner is the nearest voxel, ties to even as the processor's
        // conversion rounds.
        let (lo, hi) = voxel_light_box(Vec3::new(10.5, 10.5, 30.5), BLOCK).unwrap();
        assert_eq!((lo.x, hi.x), (8, 13));
        let (lo, hi) = voxel_light_box(Vec3::new(11.5, 11.5, 30.5), BLOCK).unwrap();
        assert_eq!((lo.x, hi.x), (9, 15));
        let (lo, hi) = voxel_light_box(Vec3::new(12.5, 12.5, 30.5), BLOCK).unwrap();
        assert_eq!((lo.x, hi.x), (9, 15));
    }
}
