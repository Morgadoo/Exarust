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
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BindGroupLayoutEntry,
    BindingType, CachedComputePipelineId, CompareFunction, ComputePassDescriptor,
    ComputePipelineDescriptor, Extent3d, FilterMode, PipelineCache, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderStages, ShaderType, StorageTextureAccess, TexelCopyTextureInfo,
    Texture, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, UniformBuffer,
};
use bevy::render::render_resource::{Origin3d, TexelCopyBufferLayout};
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
                crate::gi_shape::GiShapePlugin,
                ExtractResourcePlugin::<GiTextures>::default(),
                ExtractResourcePlugin::<GiPendingTextures>::default(),
                ExtractResourcePlugin::<GiOccupancyPatches>::default(),
                ExtractResourcePlugin::<GiLights>::default(),
                ExtractResourcePlugin::<GiSunShadow>::default(),
                ExtractResourcePlugin::<GiSunSky>::default(),
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
                (apply_occupancy_patches, prepare_bind_group)
                    .chain()
                    .in_set(RenderSystems::PrepareBindGroups),
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
    /// Whether the steps reach the top slice (the environment record's
    /// flag `0x20`); otherwise it is cleared every step.
    pub top_slice: bool,
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
    /// Whether the steps reach the top slice.
    top_slice: bool,
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

    /// The origin a surface samples the light volume from, for
    /// `texture(p·scale + offset)` with `p` in world metres: the block's
    /// lowest corner, less half a voxel in height. The original's per-object
    /// lookup adds half a slice to the height coordinate
    /// (`(below + 0.5)/depth`) so a surface reads its voxel's centre; the
    /// terrain and grass paths pass the flag that drops it, and use
    /// [`GiTextures::sample_origin_floor`] instead.
    pub fn sample_origin(&self) -> Vec4 {
        (self.origin - Vec3::Y * (0.5 * self.voxel)).extend(1.0)
    }

    /// The origin for the draw paths that read the voxel's bottom rather
    /// than its centre (the original's terrain and grass callers).
    pub fn sample_origin_floor(&self) -> Vec4 {
        self.origin.extend(1.0)
    }

    /// The scale a surface samples the light volume with: one texture unit
    /// per voxel width horizontally and per voxel height vertically.
    pub fn sample_extent(&self) -> Vec4 {
        self.extent.extend(0.0)
    }
}

/// The next block's textures, warming while the previous block stays
/// active. A block rebuilt as the camera moves starts black; swapping it in
/// before its warm-up has run leaves hard black patches and halos on the
/// floor. The old block stays bound until the new one is warm.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiPendingTextures(pub GiTextures);

/// One changed sub-rectangle of a block's occupancy, for a sub-upload: the
/// block's first 6 m cell, the region in texture axes (X, Z depth, Y height)
/// and its bytes, X fastest, then Z, then height.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GiOccupancyPatch {
    pub sector: IVec2,
    pub lo: UVec3,
    pub hi: UVec3,
    pub bytes: Vec<u8>,
}

/// The occupancy patches the world found this frame (a moving mesh took its
/// old points out and put its new ones in). The world drains the list every
/// frame, so each patch is applied once.
#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiOccupancyPatches(pub Vec<GiOccupancyPatch>);

/// A light's share of the GI injection (its record's multiplier); lights
/// without one inject in full.
#[derive(Component, Clone, Copy, Debug)]
pub struct GiMultiplier(pub f32);

/// The radius the GI injection falls off over: the light record's stored
/// radius, whatever its range flag. The injection reads it directly, so an
/// unranged light still injects over that radius.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct GiRadius(pub f32);

/// A light's dimming, as its record stores it: the injection multiplies the
/// light by (1 − dim)². Shipped point lights leave it at 0.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct GiDim(pub f32);

/// A light's GI shadow bias: the original's runtime light bias, the shadow
/// detail plus the light record's own quality offset (capped at 6) times
/// 0.005, or 0.025 above 5. The GI's cube lookup scales the receiver's
/// distance by `bias + 0.94`, so the shipped detail 5 with no offset gives
/// the 0.965 factor. See [`gi_shadow_bias`].
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct GiShadowBias(pub f32);

impl Default for GiShadowBias {
    fn default() -> Self {
        Self(gi_shadow_bias(0))
    }
}

/// The retail settings' shadow detail the original's GI cube bias is
/// derived from: 5, the top setting. Exarust has no ShadowDetail option of
/// its own, so this stays the retail value.
pub const GI_SHADOW_DETAIL: i32 = 5;

/// The original's GI shadow bias for a light whose record keeps the
/// shadow-quality offset `offset` (0, +1, −1 or −2 in the flags' low two
/// bits): `q = detail + offset`, capped at 6, then `q·0.005`, or 0.025 when
/// q is above 5. The retail detail 5 gives 0.025 for offsets 0 and +1, so
/// the cube compare is d·0.965/R.
pub fn gi_shadow_bias(offset: i32) -> f32 {
    let q = (GI_SHADOW_DETAIL + offset).min(6);
    if q > 5 {
        0.025
    } else {
        // The product is taken in double precision and rounded once.
        (f64::from(q) * 0.005) as f32
    }
}

/// Whether the light's range flag is on (with a positive radius). An unranged
/// light floods with unlimited reach and injects over its stored radius; its
/// record's shadow far plane is then 3.4e38 instead of the radius, which the
/// GI's cube lookup scale divides by. [guess: the original's cube faces are
/// drawn against the same far plane and hold ≈ 0, so its unranged taps would
/// be shadowed wherever geometry is; Exarust's faces hold distance / R, so
/// here the tiny scale leaves them lit.]
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GiRanged(pub bool);

/// The id the sun has in the GI's light sets. Point lights count up from 0,
/// so the largest value never names one.
pub const SUN_LIGHT_ID: u32 = u32::MAX;

/// The sun's fitted shadow matrix (world metres to Bevy's clip space) and
/// whether it may be sampled. The world's sun-shadow fit writes it; without
/// it the sun still lights the volume, unshadowed.
#[derive(Resource, Clone, Copy, Debug, PartialEq, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiSunShadow {
    pub clip_from_world: Mat4,
    pub enabled: bool,
}

impl Default for GiSunShadow {
    fn default() -> Self {
        Self {
            clip_from_world: Mat4::IDENTITY,
            enabled: false,
        }
    }
}

/// Whether the level has an open sky: the original's sun branch then takes
/// the shadow map's first sample alone rather than its two-sample
/// difference.
#[derive(Resource, Clone, Copy, Debug, Default, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiSunSky(pub bool);

/// A GI light shape (the original's abstract `TGIEmissive` base: a dome, a
/// cuboid, a bubble, a line or a multi-point shape). Gameplay effects
/// register a shape with the GI manager, and each subclass builds its own
/// injection program. The base's only virtual slot does nothing, so the
/// default injection adds nothing; Exarust's point and voxel lights are the
/// shapes it draws today (`gi.wesl`).
pub trait GiEmissive {
    /// Whether this shape injects light this frame. The base returns
    /// `false`, as its stub returns 0.
    fn injects(&self) -> bool {
        false
    }
}

/// A GI light shape that does nothing (the abstract base's stand-in).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoGiShape;

impl GiEmissive for NoGiShape {}

/// A voxel light (the original's voxel-light node: a salve's, a power
/// crystal's or embers' glow). Every step it adds its colour straight into
/// the voxels around it; it never lights a surface directly.
///
/// The original's node keeps the base colour, the current intensity, the
/// derived linear colour, a node flag word, the active radius, and its base
/// radius and intensity. The constructor only sets colour 1 and intensity
/// 1; the file reader then fills the record's values, and activation copies
/// the base radius to the active one.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct VoxelLight {
    /// The node's sRGB colour.
    pub color: [f32; 3],
    /// The current intensity: the colour counts by its square root.
    pub intensity: f32,
    /// The record's intensity; effects scale it to set the current one.
    pub base_intensity: f32,
    /// The base radius, metres.
    pub radius: f32,
    /// The active radius, metres; activation copies the base radius here.
    pub active_radius: f32,
    /// lin(colour)·√intensity·3 (×0.6 on the legacy path).
    pub colour: Vec3,
    /// The node flag word ([`voxel_flags`]): the colour-changed and the
    /// activated bits.
    pub flags: u32,
}

/// The node flag word's bits a voxel light sets.
pub mod voxel_flags {
    /// The colour was recomputed ([`VoxelLight::colour_update`]).
    pub const COLOUR_DIRTY: u32 = 4;
    /// The light is active ([`VoxelLight::activate`]).
    pub const ACTIVE: u32 = 2;
}

impl VoxelLight {
    /// The light of a node's sRGB colour and intensity: the intensity
    /// counts by its square root, ×3 in the original's linear workflow.
    pub fn new(srgb: [f32; 3], intensity: f32) -> Self {
        let mut light = Self {
            color: srgb,
            intensity,
            base_intensity: intensity,
            radius: 0.0,
            active_radius: 0.0,
            colour: Vec3::ZERO,
            flags: 0,
        };
        light.colour_update(false);
        light
    }

    /// A light loaded from a node's record: its colour, intensity and
    /// stored radius, active from the start (the reader fills the node's
    /// fields and the node is then added to its sector).
    pub fn from_record(srgb: [f32; 3], intensity: f32, radius: f32) -> Self {
        let mut light = Self::new(srgb, intensity);
        light.radius = radius;
        light.activate();
        light
    }

    /// The colour the original derives for `srgb` and `intensity`: the
    /// linear colour times √intensity × 3, or on its legacy path times 0.6
    /// in double precision.
    pub fn colour_for(srgb: [f32; 3], intensity: f32, legacy: bool) -> Vec3 {
        let linear = linear_rgb(srgb);
        let root = intensity.max(0.0).sqrt();
        let gain = if legacy {
            // SQRTSS, then a double-precision ×0.6, then back to float.
            (f64::from(root) * 0.6) as f32
        } else {
            root * VOXEL_LIGHT_GAIN
        };
        linear * gain
    }

    /// Recomputes the derived colour from the node's colour and current
    /// intensity, as the original's colour update does, and marks the node's
    /// colour dirty. The colour update also clears an unread field of the
    /// node, kept as no field here.
    pub fn colour_update(&mut self, legacy: bool) {
        self.colour = Self::colour_for(self.color, self.intensity, legacy);
        self.flags |= voxel_flags::COLOUR_DIRTY;
    }

    /// Activates the light: recomputes the colour, copies the base radius
    /// to the active radius and marks the node active.
    pub fn activate(&mut self) {
        self.colour_update(false);
        self.active_radius = self.radius;
        self.flags |= voxel_flags::ACTIVE;
    }

    /// Changes the current intensity at run time, as the original's effects
    /// do (they write the node's intensity, typically the base intensity
    /// times a factor, and call the colour update). The base intensity is
    /// the record's value and does not move. The colour is recomputed on the
    /// modern path.
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
        self.colour_update(false);
    }

    /// A copy of another node's values, as the original's node clone copies
    /// them: colour, current intensity, base intensity, radii and the
    /// derived linear colour. The clone's flags are its own.
    pub fn copied_from(&self) -> Self {
        Self {
            color: self.color,
            intensity: self.intensity,
            base_intensity: self.base_intensity,
            radius: self.radius,
            active_radius: self.active_radius,
            colour: self.colour,
            flags: 0,
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
            out.push(id);
        }
        out
    }

    /// Adds a light at the end, unless the list is full.
    pub fn push(&mut self, id: u32) {
        if (self.count as usize) < MAX_SECTOR_LIGHTS {
            self.lights[self.count as usize] = id;
            self.count += 1;
        }
    }

    /// The sector's light ids, in the cell's order.
    pub fn ids(&self) -> &[u32] {
        &self.lights[..self.count as usize]
    }

    /// Whether the list holds `id`.
    pub fn contains(&self, id: u32) -> bool {
        self.ids().contains(&id)
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
    /// The block these sectors belong to: its first 6 m cell in the level's
    /// grid.
    pub block: IVec2,
    /// The block's first cell counted from the grid's middle cell (the
    /// grid's size halved, rounded down), which puts its cells' centres in
    /// the world.
    pub from_middle: IVec2,
    /// Whether the level has a sector grid. Without one the original draws
    /// all 64 sectors as one group, from the bottom slice, with the scene's
    /// lights; the marks are not read.
    pub grid: bool,
    /// Sectors drawn this frame, X fastest (texture X, then Z).
    pub drawn: [bool; SECTORS * SECTORS],
    /// Each sector's lights.
    pub sectors: [GiSector; SECTORS * SECTORS],
}

impl Default for GiSectors {
    fn default() -> Self {
        Self {
            block: IVec2::ZERO,
            from_middle: IVec2::ZERO,
            grid: true,
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
    /// The lights its cell records that reach its floor.
    pub lights: GiSector,
}

/// A group of sectors the original draws together.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GiGroupPlan {
    /// The lowest slice any sector of the group updates.
    pub floor: u8,
    /// The group's lights, in the order its sectors list them (a merge
    /// keeps at most twelve, so they always fit).
    pub lights: GiSector,
    /// How many sectors the group draws.
    pub sectors: u8,
}

impl GiGroupPlan {
    /// The original's fill estimate for drawing `sectors` sectors with
    /// `lights` lights from slice `floor` up to below the top of a block
    /// `depth` slices high, in its 32-bit arithmetic.
    fn cost(lights: usize, sectors: u8, floor: u8, depth: u32) -> i32 {
        let slices = (depth as i32)
            .wrapping_sub(1)
            .wrapping_sub(i32::from(floor));
        (lights as i32 + 2)
            .wrapping_mul(i32::from(sectors))
            .wrapping_mul(slices)
            .wrapping_add(80)
    }
}

/// The original's sector grouping for a block `depth` slices high, into
/// `groups` (cleared first). A marked sector with geometry joins the first
/// group whose light list holds the same lights (as many, each found) and
/// whose floor is within two slices, and lowers its floor; otherwise it
/// starts a group. Then, in two passes over the pairs, group `j` folds into
/// an earlier group `i` when they share a light (or both have none), their
/// union stays within twelve lights, and the merged group's fill estimate
/// is below the two apart. Returns each sector's group, or 255 for a
/// sector not drawn.
pub fn group_sectors(
    sectors: &[GiSectorPlan; SECTORS * SECTORS],
    depth: u32,
    groups: &mut Vec<GiGroupPlan>,
) -> [u8; SECTORS * SECTORS] {
    groups.clear();
    let mut of_sector = [u8::MAX; SECTORS * SECTORS];
    for (i, sector) in sectors.iter().enumerate() {
        // A sector the camera did not mark is left out, and so is one at or
        // above the block's top (a sector without geometry has floor 255).
        if !sector.drawn || u32::from(sector.floor) >= depth {
            continue;
        }
        let ids = sector.lights.ids();
        let joined = groups.iter().position(|g| {
            g.lights.ids().len() == ids.len()
                && (i32::from(g.floor) - i32::from(sector.floor)).abs() < 3
                && ids.iter().all(|&id| g.lights.contains(id))
        });
        match joined {
            Some(g) => {
                let group = &mut groups[g];
                group.floor = group.floor.min(sector.floor);
                group.sectors += 1;
                of_sector[i] = g as u8;
            }
            None => {
                groups.push(GiGroupPlan {
                    floor: sector.floor,
                    lights: sector.lights,
                    sectors: 1,
                });
                of_sector[i] = (groups.len() - 1) as u8;
            }
        }
    }
    for _ in 0..2 {
        let mut i = 0;
        while i < groups.len() {
            let mut j = i + 1;
            while j < groups.len() {
                let (a, b) = (groups[i], groups[j]);
                let common = a
                    .lights
                    .ids()
                    .iter()
                    .filter(|&&l| b.lights.contains(l))
                    .count();
                let (na, nb) = (a.lights.ids().len(), b.lights.ids().len());
                if common == 0 && (nb > 0 || na > 0) {
                    j += 1;
                    continue;
                }
                let union = nb + (na - common);
                if union > MAX_SECTOR_LIGHTS {
                    j += 1;
                    continue;
                }
                let apart = i64::from(GiGroupPlan::cost(na, a.sectors, a.floor, depth))
                    + i64::from(GiGroupPlan::cost(nb, b.sectors, b.floor, depth));
                let floor = a.floor.min(b.floor);
                let merged = GiGroupPlan::cost(union, a.sectors + b.sectors, floor, depth);
                if apart <= i64::from(merged) {
                    j += 1;
                    continue;
                }
                // The second group's lights not already in the first join
                // it, then its sectors and its floor, and it disappears.
                let group = &mut groups[i];
                for &l in b.lights.ids() {
                    if !a.lights.contains(l) {
                        group.lights.push(l);
                    }
                }
                group.sectors += b.sectors;
                group.floor = floor;
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
    of_sector
}

/// The lights injected into the volume this frame.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GiLights {
    /// The groups' lights, one run of [`GiGroupUniform::count`] after
    /// another.
    lights: Vec<GiLight>,
    /// The cached cube slot of each light in `lights` (u32::MAX for none).
    /// The original binds a light's cube map to `Shadow<i>`, one sampler per
    /// light index; Exarust's one cube pool needs the slot beside the light.
    cube_slots: Vec<u32>,
    groups: Vec<GiGroupUniform>,
    /// Per sector (X fastest), its group or 255.
    sector_groups: [u8; SECTORS * SECTORS],
    voxel_lights: Vec<GiVoxelLight>,
}

impl Default for GiLights {
    fn default() -> Self {
        Self {
            lights: Vec::new(),
            cube_slots: Vec::new(),
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

/// One light as the injection's `Light[3i..3i+2]` uniforms: the position and
/// inverse radius in texture units, the colour and the cube-shadow depth
/// scale, and the vertical shaping.
#[derive(Clone, Copy, Default, ShaderType)]
struct GiLight {
    /// A point light at (x, z depth, y height) in texture units, its inverse
    /// radius in them in `w`; for the sun only `w` is read (unused).
    position_radius: Vec4,
    /// The injected colour in `rgb`; `w` is the cube lookup's depth scale
    /// ((bias + 0.94)/R × 5120, the original's 5120 = 256 × 20 cm).
    colour_shadow: Vec4,
    /// x, y: vertical shaping (y, z); z: 1/(2^z − 1); w: 0 for a point
    /// light and 1 for the sun. The original keeps the shaping one lane up
    /// (y, z, w of `Light[3i+2]`, x unwritten) and splits the sun into its
    /// own program branch.
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
    /// The sun's fitted shadow matrix (world metres to clip space).
    sun_matrix: Mat4,
    /// x: the shadow map may be sampled, y: the level has a sky, z: the
    /// steps reach the top slice.
    sun_flags: UVec4,
    /// The groups of sectors drawn this frame.
    groups: [GiGroupUniform; MAX_GROUPS],
    /// Per sector (X fastest), its group or u32::MAX.
    sector_groups: [UVec4; SECTORS * SECTORS / 4],
    lights: [GiLight; MAX_GROUP_LIGHTS],
    /// The cached cube slot of each light, `u32::MAX` for none: the
    /// original binds each light's cube map to its own `Shadow<i>` sampler.
    cubes: [UVec4; MAX_GROUP_LIGHTS / 4],
    /// x = the number of voxel lights.
    voxel_count: UVec4,
    voxel_lights: [GiVoxelLight; MAX_VOXEL_LIGHTS],
}

/// Lumens per unit of the original's light intensity (see `exa_world`).
const LUMENS_PER_INTENSITY: f32 = 8.0;
/// Lux per unit of the original's sun intensity (see `exa_world`).
const SUN_LUX: f32 = 160.0;
/// The manager's sun factor: the scene's sun colour times 3 is the GI sun.
const SUN_GI_SCALE: f32 = 3.0;
/// The manager's injection share; point lights take 1.5 times it, the sun 3.
const INJECTION_SHARE: f32 = 0.1;

/// The manager's light-injection share for a shading quality: 0.1, or 0.09
/// at the high quality (above 3), which is what the original's per-frame
/// entry stores in the field its light sources read. The GI's own point and
/// sun uniforms use the fixed 0.1, as the original's per-block injection
/// reads the unqualified value; a source outside the GI manager (the effects'
/// direct light) takes this one.
pub fn injection_share(high_quality: bool) -> f32 {
    if high_quality {
        INJECTION_SHARE * 0.9
    } else {
        INJECTION_SHARE
    }
}
/// The sun's intensity below which the original leaves it out of the GI.
const SUN_EPSILON: f32 = 1.192_09e-7;
/// The original's own sun shadow map for the GI: 1024², with its direct-light
/// softness of 1600 (the sun filter radius). Exarust samples Bevy's fitted sun map instead, so these
/// stay as the original's numbers for reference.
pub const GI_SUN_SHADOW_SIZE: u32 = 1024;
pub const GI_SUN_SOFTNESS: f32 = 1600.0;
/// Light ranges are 1.25 radii; the volume falls off over one radius.
const REACH: f32 = 1.25;
/// The shadow far plane an unranged light's record holds, 3.4e38, as the
/// original has it (see [`GiRanged`] for what the GI lookup makes of it).
const UNRANGED_FAR: u32 = 0x7f7f_c99e;
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

/// The manager's block presets: the voxel size picks the volume's size and
/// the level's height its slice count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GiPreset {
    /// Voxels across (X and Z).
    pub width: u32,
    pub height: u32,
    /// Slices.
    pub depth: u32,
    /// Voxel edge, centimetres.
    pub voxel_cm: u32,
}

/// The original's preset for a voxel edge: 5 cm gives 32 × 32 × 64, 20 cm
/// gives 256 × 256 × 64 (128 when the level is more than 12.8 m tall), and
/// every other value 128 × 128 × 16 at 40 cm. The preset and the level's
/// height are in whole centimetres, divided as integers.
pub fn preset(voxel_cm: i32, level_span_cm: i32) -> GiPreset {
    match voxel_cm {
        5 => GiPreset {
            width: 32,
            height: 32,
            depth: 64,
            voxel_cm: 5,
        },
        20 => GiPreset {
            width: 256,
            height: 256,
            depth: if level_span_cm / 20 > 64 { 128 } else { 64 },
            voxel_cm: 20,
        },
        _ => GiPreset {
            width: 128,
            height: 128,
            depth: 16,
            voxel_cm: 40,
        },
    }
}

/// The eight diagonal corner offsets the propagation samples, in texture
/// units, with `w` = 1/8. The original's order is X fastest, then Z, then Y,
/// with the zero corner left out.
pub fn corner_offsets(size: UVec3) -> [[f32; 4]; 8] {
    // The original works out each axis in double precision from the integer
    // corner (-1 or 1), the double 1.33, and the integer size, then rounds
    // once to f32; the f32 product would lose the last bit on some sizes.
    let step = |corner: i32, n: u32| ((f64::from(corner) * 1.33) / f64::from(n.max(1))) as f32;
    let mut out = [[0.0; 4]; 8];
    let mut i = 0;
    for z in [-1, 1] {
        for y in [-1, 1] {
            for x in [-1, 1] {
                out[i] = [step(x, size.x), step(y, size.y), step(z, size.z), 1.0 / 8.0];
                i += 1;
            }
        }
    }
    out
}

/// The quad index buffer the original's draws share: 4·i, 4·i+1, 4·i+2,
/// then 4·i, 4·i+2, 4·i+3.
pub fn quad_indices(quads: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(quads * 6);
    for q in 0..quads {
        let b = (q * 4) as u16;
        out.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    out
}

pub use crate::gi_shape::{GiShape, GiShapeProgram, GiShapeSample, GiShapes};

/// A registered shape's volume for the manager's box: its world position
/// and half extents (metres), in world X, Y, Z. A shape entity carries one
/// and is listed in [`GiShapes`]; the effect classes outside this component
/// add their own uniforms and fragment program.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ShapeVolume {
    pub position: Vec3,
    pub half: Vec3,
}

/// The layers and column range the manager draws a shape over, in voxels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeDraw {
    /// First and one-past-last column (texture X, then Z).
    pub lo: UVec2,
    pub hi: UVec2,
    /// First slice and how many.
    pub first: i32,
    pub count: i32,
}

/// The region the manager draws a shape's injection over, as its shape loop
/// computes it: the centre in voxels (CVTSS2SI, ties to even), the half
/// extents rounded up (the engine's ceiling helper), the columns clipped to
/// voxels 8 and 248 (the original's fixed bounds, the 8-voxel margins of a
/// 256-wide block), and the slices from the centre less its vertical half
/// extent, one more than twice it. Shapes whose centre is below slice 2, or
/// whose clipped box is empty, draw nothing.
///
/// `dims` are the block's voxels in texture axes (X, Z depth, Y height); the
/// manager's scale is one over the width times a voxel, and each centre is
/// its voxel index: `round(width · ((p − block centre) · scale + 0.5))`,
/// worked out in single precision in that order. The vertical centre is the
/// world's zero (`below/width + p·scale`), and the half extents use the
/// width on every axis, as the original does (all three presets are square).
pub fn shape_draw(volume: ShapeVolume, origin: Vec3, dims: UVec3, voxel: f32) -> Option<ShapeDraw> {
    let width = dims.x.max(1) as f32;
    let inv = 1.0 / (width * voxel);
    let centre = Vec3::new(
        origin.x + 0.5 * width * voxel,
        origin.y + 0.5 * dims.z as f32 * voxel,
        origin.z + 0.5 * dims.y as f32 * voxel,
    );
    // X and Z measure from the block's centre; the height measures from the
    // world's zero (the block's vertical origin is its `below` offset).
    let below = -origin.y / voxel;
    let column = |p: f32, c: f32| (p - c) * inv + 0.5;
    let centre_x = (width * column(volume.position.x, centre.x)).round_ties_even() as i32;
    let centre_z =
        (dims.y.max(1) as f32 * column(volume.position.z, centre.z)).round_ties_even() as i32;
    let centre_y = (width * (below / width + volume.position.y * inv)).round_ties_even() as i32;
    let half = |h: f32| (width * h * inv).ceil() as i32;
    let half_x = half(volume.half.x);
    let half_z = half(volume.half.z);
    let half_y = half(volume.half.y);
    let lo =
        (IVec2::new(centre_x, centre_z) - IVec2::new(half_x, half_z)).max(IVec2::splat(SHAPE_LO));
    let hi =
        (IVec2::new(centre_x, centre_z) + IVec2::new(half_x, half_z)).min(IVec2::splat(SHAPE_HI));
    (lo.x < hi.x && lo.y < hi.y && centre_y > 1).then_some(ShapeDraw {
        lo: lo.as_uvec2(),
        hi: hi.as_uvec2(),
        first: centre_y - half_y,
        count: half_y * 2 + 1,
    })
}

/// The first and one-past-last column a shape draws over (both axes).
const SHAPE_LO: i32 = 8;
const SHAPE_HI: i32 = 248;

/// The four corners of one shape injection quad, in texture units (x,
/// Z depth, Y height), as the manager's shape loop builds them: the shape's
/// clipped column box in x and y, and r at the group's first slice plus half
/// a slice (the vertex shader adds each instance's layer). The quad walks the
/// box's edges anticlockwise, as the original's four vertices do.
pub fn shape_quad(draw: &ShapeDraw, dims: UVec3) -> [Vec3; 4] {
    let w = dims.x.max(1) as f32;
    let h = dims.y.max(1) as f32;
    let d = dims.z.max(1) as f32;
    let (x0, x1) = (draw.lo.x as f32, draw.hi.x as f32);
    let (y0, y1) = (draw.lo.y as f32, draw.hi.y as f32);
    // The base quad's z is `-(first + 0.5)/depth`, the first slice's centre
    // negated; each instance adds its own layer.
    let r = -((draw.first as f32 + 0.5) / d);
    [
        Vec3::new(x0 / w, y0 / h, r),
        Vec3::new(x1 / w, y0 / h, r),
        Vec3::new(x1 / w, y1 / h, r),
        Vec3::new(x0 / w, y1 / h, r),
    ]
}

/// The uniforms every shape program takes from the manager, for a block of
/// `dims` texels: `SliceSz` (one over the slice count) and `ZTrn` (−D/W,
/// D/W). The manager sets them once per program before the shape's own
/// uniforms, as do the voxel-light and propagation paths.
pub fn shape_uniforms(dims: UVec3) -> (f32, Vec2) {
    let w = dims.x.max(1) as f32;
    let d = dims.z.max(1) as f32;
    (1.0 / d, Vec2::new(-d / w, d / w))
}

/// One of the manager's linked programs: the vertex, geometry and fragment
/// entries it combines, the textures and samplers it binds, and the
/// uniforms it looks up. The original builds and caches one program per
/// purpose; Exarust's renderer replaces them with the WESL compute pass
/// (`gi.wesl`), so this keeps the original's binding table exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GiProgramLayout {
    /// The vertex entry's name in the shader library.
    pub vertex: &'static str,
    /// The geometry entry's name.
    pub geometry: &'static str,
    /// The fragment entry's name.
    pub fragment: &'static str,
    /// The sampler uniforms the manager sets and their texture units, in
    /// the order it sets them (a sampler it never sets stays on unit 0).
    pub samplers: &'static [(&'static str, u32)],
    /// The other uniforms the manager looks up, in the order it looks them
    /// up.
    pub uniforms: &'static [&'static str],
}

/// The three programs the manager links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GiPass {
    /// `gi_inject_v`/`gi_inject_g` with `gi_inject_f`: one slice of the
    /// propagation and direct light.
    Inject,
    /// The same vertex and geometry with `gi_emission`: the voxel lights,
    /// added after the propagation (the light shapes link their own
    /// fragments through the same builder, which binds `MPrj` and `OVox`).
    Emission,
    /// The same vertex and geometry with `gi_blit_f`: one neighbour block's
    /// margin copied into this block.
    Blit,
}

impl GiPass {
    /// The pass's program and binding table.
    pub fn layout(self) -> GiProgramLayout {
        match self {
            Self::Inject => GiProgramLayout {
                vertex: "gi_inject_v",
                geometry: "gi_inject_g",
                fragment: "gi_inject_f",
                // One `Shadow<i>` per light in the group (up to twelve),
                // set before the two volumes.
                samplers: &[
                    ("Shadow0", GI_SHADOW_UNIT),
                    ("Shadow1", GI_SHADOW_UNIT + 1),
                    ("Shadow2", GI_SHADOW_UNIT + 2),
                    ("Shadow3", GI_SHADOW_UNIT + 3),
                    ("Shadow4", GI_SHADOW_UNIT + 4),
                    ("Shadow5", GI_SHADOW_UNIT + 5),
                    ("Shadow6", GI_SHADOW_UNIT + 6),
                    ("Shadow7", GI_SHADOW_UNIT + 7),
                    ("Shadow8", GI_SHADOW_UNIT + 8),
                    ("Shadow9", GI_SHADOW_UNIT + 9),
                    ("Shadow10", GI_SHADOW_UNIT + 10),
                    ("Shadow11", GI_SHADOW_UNIT + 11),
                    ("LVox", GI_LVOX_UNIT),
                    ("OVox", GI_OVOX_UNIT),
                ],
                uniforms: &[
                    "MPrj", "First", "Light", "VSmp", "ZTrn", "LyrOfs", "BaseAmb", "WrlMat",
                    "SdwMat", "SdwMat2", "SliceSz",
                ],
            },
            Self::Emission => GiProgramLayout {
                vertex: "gi_inject_v",
                geometry: "gi_inject_g",
                fragment: "gi_emission",
                samplers: &[("OVox", GI_OVOX_UNIT)],
                uniforms: &["MPrj", "SliceSz", "ZTrn", "First", "Light"],
            },
            Self::Blit => GiProgramLayout {
                vertex: "gi_inject_v",
                geometry: "gi_inject_g",
                fragment: "gi_blit_f",
                samplers: &[("LVox", GI_LVOX_UNIT)],
                uniforms: &["MPrj", "First", "UVOfs", "SliceSz"],
            },
        }
    }
}

/// The original's texture units in the injection and blit programs.
pub const GI_LVOX_UNIT: u32 = 0;
pub const GI_OVOX_UNIT: u32 = 1;
/// The first shadow sampler: light `i` binds `Shadow<i>` to unit
/// [`GI_SHADOW_UNIT`] + `i`, right after `LVox` and `OVox`.
pub const GI_SHADOW_UNIT: u32 = 2;

/// The sampler uniform `Shadow<i>` uses for light `i` (0..12), and the
/// texture unit it binds. The manager patches the digits of `Shadow0` (a
/// `1` and the last digit from ten up) and sets each to `2 + i`; a group
/// holds at most twelve lights.
pub fn shadow_sampler(light: usize) -> (String, u32) {
    // The digit patch spells 0..=19 the same as decimal.
    (format!("Shadow{light}"), GI_SHADOW_UNIT + light as u32)
}

/// One margin copy between a block and a neighbour: the destination columns
/// and the offset added to them to read the neighbour's light texture. The
/// manager copies every slice but the top one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginBlit {
    /// Destination columns (texture X, then Z), half open.
    pub lo: UVec2,
    pub hi: UVec2,
    /// Added to the destination's texture coordinates to read the source.
    pub offset: Vec2,
    /// Slices to copy.
    pub slices: u32,
}

/// The margin copy from block `(dx, dz)`'s neighbour (each −1, 0 or 1, not
/// both 0): a one-sector margin column or row on that side, the interior on
/// the other axis, and the original's UV offset of ∓240/256. A diagonal
/// neighbour fills both margins.
pub fn margin_blit(dims: UVec3, dx: i32, dz: i32) -> Option<MarginBlit> {
    if dx == 0 && dz == 0 {
        return None;
    }
    let m = 8u32;
    let columns = |d: i32, size: u32| match d.signum() {
        -1 => 0..m,
        1 => size - m..size,
        _ => m..size - m,
    };
    let xs = columns(dx, dims.x);
    let zs = columns(dz, dims.y);
    let fraction = (dims.x as f32 - 16.0) / dims.x as f32;
    Some(MarginBlit {
        lo: UVec2::new(xs.start, zs.start),
        hi: UVec2::new(xs.end, zs.end),
        offset: Vec2::new(-dx as f32 * fraction, -dz as f32 * fraction),
        slices: dims.z.saturating_sub(1),
    })
}

/// The four corners of a margin blit's quad, in destination texture units
/// (x, Z depth, Y height), with the base z the original's quad carries
/// (−0.5/depth; each instance's layer adds 1/depth). The fragment reads the
/// neighbour at the same coordinates plus [`MarginBlit::offset`]. The quad
/// walks the destination box's edges anticlockwise.
pub fn margin_quad(blit: &MarginBlit, dims: UVec3) -> [Vec3; 4] {
    let w = dims.x.max(1) as f32;
    let h = dims.y.max(1) as f32;
    let d = dims.z.max(1) as f32;
    let (x0, x1) = (blit.lo.x as f32 / w, blit.hi.x as f32 / w);
    let (y0, y1) = (blit.lo.y as f32 / h, blit.hi.y as f32 / h);
    let r = -0.5 / d;
    [
        Vec3::new(x0, y0, r),
        Vec3::new(x1, y0, r),
        Vec3::new(x1, y1, r),
        Vec3::new(x0, y1, r),
    ]
}

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

/// A voxel's texture coordinate as the injection reads it: the texel centre
/// of its column and the centre of its *own* slice, since the original's
/// vertex shader maps layer L to `r = (L + 0.5)/depth` (the quad's vertex r
/// is −0.5/depth and the layer offset is subtracted). Texture axes are X,
/// Z depth, Y height. Mirrors `volume_uv` in `gi.wesl`.
pub fn voxel_uv(id: UVec3, dims: UVec3) -> Vec3 {
    let d = dims.max(UVec3::ONE).as_vec3();
    (id.as_vec3() + Vec3::splat(0.5)) / Vec3::new(d.x, d.y, d.z)
}

/// A voxel's position in the injection's world-scale texture units: x and
/// z depth as texture units, the height over the block's **width** (not its
/// slice count), as the original's `ZTrn` maps it. Mirrors `volume_height`
/// in `gi.wesl`.
pub fn voxel_height(id: UVec3, dims: UVec3) -> Vec3 {
    let d = dims.max(UVec3::ONE).as_vec3();
    Vec3::new(
        (id.x as f32 + 0.5) / d.x,
        (id.y as f32 + 0.5) / d.y,
        (id.z as f32 + 0.5) / d.x,
    )
}

/// The slices a sector group's light step draws: from its floor (the
/// original's `LyrOfs`) up to the volume's depth, less the top slice unless
/// the level record's flag lets the step reach it (the original's instance
/// count is `D − floor` with the flag, `D − 1 − floor` without). The others
/// are cleared. Mirrors the slice test in `gi.wesl`.
pub fn drawn_slices(floor: u32, depth: u32, top_slice: bool) -> std::ops::Range<u32> {
    let top = if top_slice {
        depth
    } else {
        depth.saturating_sub(1)
    };
    floor.min(top)..top
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
        top_slice: volume.top_slice,
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
#[derive(Clone, Copy)]
struct LightSource {
    colour: Vec3,
    intensity: f32,
    /// The radius the injection falls off over, metres.
    radius: f32,
    position: Vec3,
    multiplier: f32,
    /// The vertical shaping (y, z); (1, 1) for a light without a spot.
    shaping: Vec2,
    /// The light's dimming, as its record stores it.
    dim: f32,
    /// The GI cube lookup's shadow bias (see [`GiShadowBias`]).
    bias: f32,
    /// The range flag: an unranged light's cube lookup is unshadowed.
    ranged: bool,
    /// The cached cube shadow's slot + 1 (0 without one).
    cube: u32,
    /// The sun, which is injected by its shadow map rather than a position.
    sun: bool,
}

impl LightSource {
    /// Whether the light has the original's spot shaping.
    fn shaped(&self) -> bool {
        self.shaping != Vec2::ONE
    }

    /// The light's per-step colour: the injection share 0.1 times 1.5 and
    /// the original's `(1 − dim)²` for a point light; for the sun its colour
    /// is first scaled by the manager's sun factor 3 and then by the share
    /// times 3 (0.9 in all). The uniform's 0.01 is still to come.
    fn injection_colour(&self) -> Vec3 {
        let colour = self.colour * self.intensity;
        if self.sun {
            return colour * SUN_GI_SCALE * (INJECTION_SHARE * 3.0);
        }
        let dim = (1.0 - self.dim) * (1.0 - self.dim);
        colour * (dim * INJECTION_SHARE * 1.5)
    }

    /// The cube lookup's depth scale: the original's
    /// `((bias + 0.94) / far) × 5120`, worked out in double precision
    /// and stored as a float. `far` is the light's radius in centimetres
    /// when its range is on, else the 3.4e38 the record holds, so an
    /// unranged light's compare value stays at the 0.01 bias (against
    /// Exarust's distance / R faces its taps are lit; see [`GiRanged`]).
    fn cube_shadow_scale(&self) -> f32 {
        let far = if self.ranged {
            f64::from(self.radius * CM_PER_M)
        } else {
            f64::from(f32::from_bits(UNRANGED_FAR))
        };
        ((f64::from(self.bias) + 0.94) / far * 5120.0) as f32
    }
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
        Option<&'static GiRadius>,
        Option<&'static crate::lights::LightShaping>,
        Option<&'static crate::cube_shadows::CubeSlot>,
        Option<&'static GiDim>,
        Option<&'static GiShadowBias>,
        Option<&'static GiRanged>,
    ),
>;

/// Centimetres to the metre: the grouping's tests run in the original's
/// units.
const CM_PER_M: f32 = 100.0;
/// A 6 m cell and half of one, and a voxel, in centimetres.
const CELL_CM: f32 = 600.0;
const HALF_CELL_CM: f32 = 300.0;
const VOXEL_CM: f32 = 20.0;
/// The shaping test's threshold: the original's constant, just below
/// `f32::EPSILON`.
const SHAPE_FLOOR: f32 = f32::from_bits(0x33ff_ffd7);

#[allow(clippy::too_many_arguments)]
fn gather_lights(
    mut gi: ResMut<GiLights>,
    lights: GiLightQuery,
    suns: Query<(&DirectionalLight, &GlobalTransform), Without<crate::supersample::Offscreen>>,
    voxel_lights: Query<(&VoxelLight, &GlobalTransform)>,
    textures: Option<Res<GiTextures>>,
    pending: Option<Res<GiPendingTextures>>,
    sectors: Option<Res<GiSectorLights>>,
    mut sources: Local<HashMap<u32, LightSource>>,
    mut groups: Local<Vec<GiGroupPlan>>,
) {
    gi.lights.clear();
    gi.cube_slots.clear();
    gi.groups.clear();
    gi.voxel_lights.clear();
    gi.sector_groups = [u8::MAX; SECTORS * SECTORS];
    let textures = pending.as_ref().map(|p| &p.0).or(textures.as_deref());
    let (Some(textures), Some(sectors)) = (textures, sectors) else {
        return;
    };
    gather_voxel_lights(&mut gi.voxel_lights, textures, voxel_lights.iter());
    let sectors = &*sectors.0;
    // Marks made for another block (the frame a new block arrives) light
    // nothing; the next frame's are its own.
    if sectors.block != textures.sector {
        return;
    }
    // The lights the cells record, by their shader id.
    sources.clear();
    for (light, at, id, share, radius, shaping, cube, dim, bias, ranged) in &lights {
        let c = light.color.to_linear();
        sources.insert(
            id.0,
            LightSource {
                colour: Vec3::new(c.red, c.green, c.blue),
                intensity: light.intensity / LUMENS_PER_INTENSITY,
                radius: radius.map_or(light.range / REACH, |r| r.0),
                position: at.translation(),
                multiplier: share.map_or(1.0, |m| m.0),
                shaping: shaping.map_or(Vec2::ONE, |s| s.0),
                dim: dim.map_or(0.0, |d| d.0),
                bias: bias.map_or(gi_shadow_bias(0), |b| b.0),
                ranged: ranged.is_none_or(|r| r.0),
                cube: cube.and_then(|c| c.slot).map_or(0, |s| s + 1),
                sun: false,
            },
        );
    }
    // The original's sun: the scene's directional light, its colour already
    // in the original's units, injected from every marked sector's group.
    let sun = suns
        .iter()
        .next()
        .filter(|(light, _)| light.illuminance > 0.0);
    // The original tests only the scaled colour's third component.
    let sun_active = sun.is_some_and(|(light, _)| {
        let c = light.color.to_linear();
        let i = light.illuminance / SUN_LUX;
        c.blue * i * SUN_GI_SCALE > SUN_EPSILON
    });
    if sun_active && let Some((light, _)) = sun {
        let c = light.color.to_linear();
        sources.insert(
            SUN_LIGHT_ID,
            LightSource {
                colour: Vec3::new(c.red, c.green, c.blue),
                intensity: light.illuminance / SUN_LUX,
                radius: 0.0,
                position: Vec3::ZERO,
                multiplier: 1.0,
                shaping: Vec2::ONE,
                dim: 0.0,
                bias: 0.0,
                ranged: true,
                cube: 0,
                sun: true,
            },
        );
    }
    let depth = textures.dims.z;
    if !sectors.grid {
        // Without a sector grid the original draws one group of all 64
        // sectors from the bottom slice, with the scene's lights.
        groups.clear();
        let mut list = GiSector::default();
        if sun_active {
            list.push(SUN_LIGHT_ID);
        }
        for &id in sectors.sectors[0].ids() {
            list.push(id);
        }
        groups.push(GiGroupPlan {
            floor: 0,
            lights: list,
            sectors: (SECTORS * SECTORS) as u8,
        });
        let of_sector = [0; SECTORS * SECTORS];
        push_groups(
            &mut gi,
            &groups,
            &sources,
            of_sector,
            textures.origin,
            textures.extent,
        );
        return;
    }
    // Each marked sector keeps the sun and then the lights of its cell that
    // the downward shaping lets reach its floor slice, in the cell's order.
    // A light the injection does not know is kept for the grouping and
    // dropped from the draw, as the original drops a light without its GI
    // flag after grouping.
    let below = (-textures.origin.y / textures.voxel).round();
    let mut plans = [GiSectorPlan::default(); SECTORS * SECTORS];
    for z in 0..SECTORS {
        for x in 0..SECTORS {
            let i = x + z * SECTORS;
            let floor = textures.sector_floor(x, z);
            let drawn = sectors.drawn[i];
            let mut kept = GiSector::default();
            if drawn && u32::from(floor) < depth {
                if sun_active {
                    kept.push(SUN_LIGHT_ID);
                }
                let cell = Vec2::new(
                    ((sectors.from_middle.x + x as i32) as f32 + 0.5) * CELL_CM,
                    ((sectors.from_middle.y + z as i32) as f32 + 0.5) * CELL_CM,
                );
                let floor_y = VOXEL_CM * ((f32::from(floor) + 0.5) - below);
                for &id in sectors.sector(x, z).ids() {
                    match sources.get(&id) {
                        Some(source) if !source.sun && !shape_reaches(source, cell, floor_y) => {}
                        _ => kept.push(id),
                    }
                }
            }
            plans[i] = GiSectorPlan {
                drawn,
                floor,
                lights: kept,
            };
        }
    }
    let of_sector = group_sectors(&plans, depth, &mut groups);
    push_groups(
        &mut gi,
        &groups,
        &sources,
        of_sector,
        textures.origin,
        textures.extent,
    );
}

/// Lays the groups' lights out for the pass, dropping the lights the
/// injection does not know.
fn push_groups(
    gi: &mut GiLights,
    groups: &[GiGroupPlan],
    sources: &HashMap<u32, LightSource>,
    of_sector: [u8; SECTORS * SECTORS],
    origin: Vec3,
    extent: Vec3,
) {
    // `extent` is in world axes (X, height, Z): the texture's second axis
    // is world Z, its third the height over the block's width.
    let width = extent.x.max(1e-6);
    let depth = extent.z.max(1e-6);
    for group in groups {
        let first = gi.lights.len();
        for id in group.lights.ids() {
            let Some(source) = sources.get(id) else {
                continue;
            };
            let colour = source.injection_colour() * source.multiplier;
            let light = if source.sun {
                GiLight {
                    colour_shadow: colour.extend(1.0),
                    shaping: Vec4::new(1.0, 1.0, 1.0, 1.0),
                    ..default()
                }
            } else {
                let p = source.position - origin;
                let shaping = source.shaping;
                GiLight {
                    position_radius: Vec4::new(
                        p.x / width,
                        p.z / depth,
                        p.y / width,
                        width / source.radius.max(1e-6),
                    ),
                    colour_shadow: colour.extend(source.cube_shadow_scale()),
                    shaping: Vec4::new(shaping.x, shaping.y, 1.0 / (shaping.y.exp2() - 1.0), 0.0),
                }
            };
            gi.lights.push(light);
            gi.cube_slots.push(if source.cube == 0 {
                u32::MAX
            } else {
                source.cube - 1
            });
        }
        gi.groups.push(GiGroupUniform {
            floor: u32::from(group.floor),
            first: first as u32,
            count: (gi.lights.len() - first) as u32,
            _pad: 0,
        });
    }
    gi.sector_groups = of_sector;
}

/// The original's downward-shaping test for a light at a sector, in
/// centimetres: `cell` is the sector's cell centre (X, Z) and `floor_y` the
/// middle of its floor slice. A light without a spot always passes. A
/// shaped one must still carry towards that point: horizontal offsets
/// within half a cell count as zero (as does one that is not a number) and
/// larger ones are pulled half a cell in, then the shaping
/// `c + (1 − c)·y` at the cosine `c` of the drop must stay above the
/// original's threshold.
fn shape_reaches(source: &LightSource, cell: Vec2, floor_y: f32) -> bool {
    if !source.shaped() {
        return true;
    }
    let at = source.position * CM_PER_M;
    let pull = |d: f32| {
        if d.abs() > HALF_CELL_CM {
            d - d.signum() * HALF_CELL_CM
        } else {
            0.0
        }
    };
    let dx = pull(at.x - cell.x);
    let dz = pull(at.z - cell.y);
    let dy = at.y - floor_y;
    // The original sums the horizontal squares first (dz² + dx²), then adds
    // dy² to them.
    let c = dy / (dy * dy + (dz * dz + dx * dx)).sqrt();
    c + (1.0 - c) * source.shaping.x > SHAPE_FLOOR
}

/// The voxel lights that reach into the block, nearest its centre first (the
/// original injects every sector's voxel lights).
fn gather_voxel_lights<'a>(
    out: &mut Vec<GiVoxelLight>,
    textures: &GiTextures,
    lights: impl Iterator<Item = (&'a VoxelLight, &'a GlobalTransform)>,
) {
    let centre = textures.dims.as_vec3() * 0.5;
    let width = textures.extent.x.max(1e-6);
    let depth = textures.extent.z.max(1e-6);
    let mut near: Vec<(f32, GiVoxelLight)> = lights
        .filter(|(light, _)| light.colour.max_element() > 0.0)
        .filter_map(|(light, at)| {
            // World X, Y, Z to texture X, Z, Y, in voxels.
            let p = (at.translation() - textures.origin) / textures.voxel;
            let voxels = Vec3::new(p.x, p.z, p.y);
            let (lo, hi) = voxel_light_box(voxels, textures.dims)?;
            let metres = at.translation() - textures.origin;
            Some((
                voxels.distance_squared(centre),
                GiVoxelLight {
                    // The pass's world-scale texture units: the height is
                    // divided by the block's width, not its slice count.
                    position: Vec3::new(metres.x / width, metres.z / depth, metres.y / width)
                        .extend(0.0),
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
        m.params.gi_origin = textures.sample_origin();
        m.params.gi_extent = textures.sample_extent();
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
    cube_sampler: Sampler,
    sun_sampler: Sampler,
    pipeline: CachedComputePipelineId,
    /// A 1-layer depth array and a six-face cube bound while the real shadow
    /// maps are absent (before the first camera's view bindings exist).
    dummy_sun: TextureView,
    dummy_cube: TextureView,
    /// Kept alive while the views above point into them.
    _dummies: (Texture, Texture),
}

/// A depth texture and its view, for the fallback shadow maps.
fn depth_texture(
    device: &RenderDevice,
    size: u32,
    layers: u32,
    format: TextureFormat,
    dimension: TextureViewDimension,
) -> (Texture, TextureView) {
    let texture = device.create_texture(&bevy::render::render_resource::TextureDescriptor {
        label: Some("exa_gi_fallback_shadow"),
        size: Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor {
        dimension: Some(dimension),
        ..default()
    });
    (texture, view)
}

fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    assets: Res<AssetServer>,
    cache: Res<PipelineCache>,
) {
    let depth_texture_entry = |dimension| BindGroupLayoutEntry {
        binding: u32::MAX,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Texture {
            sample_type: TextureSampleType::Depth,
            view_dimension: dimension,
            multisampled: false,
        },
        count: None,
    };
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
                depth_texture_entry(TextureViewDimension::CubeArray),
                sampler(SamplerBindingType::Comparison),
                depth_texture_entry(TextureViewDimension::D2Array),
                sampler(SamplerBindingType::Comparison),
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
    let (sun_texture, dummy_sun) = depth_texture(
        &device,
        1,
        1,
        TextureFormat::Depth32Float,
        TextureViewDimension::D2Array,
    );
    let (cube_texture, dummy_cube) = depth_texture(
        &device,
        1,
        6,
        TextureFormat::Depth16Unorm,
        TextureViewDimension::CubeArray,
    );
    commands.insert_resource(GiPipeline {
        layout,
        sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        // The cached cubes' sampler (lit where the receiver is nearer).
        cube_sampler: device.create_sampler(&SamplerDescriptor {
            compare: Some(CompareFunction::Less),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        // Bevy's sun map is reverse-Z, compared greater-or-equal.
        sun_sampler: device.create_sampler(&SamplerDescriptor {
            compare: Some(CompareFunction::GreaterEqual),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        pipeline,
        dummy_sun,
        dummy_cube,
        _dummies: (sun_texture, cube_texture),
    });
}

/// The pass's bind group (it holds the parameter buffer).
#[derive(Resource)]
struct GiBindGroup {
    bind_group: bevy::render::render_resource::BindGroup,
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

/// Writes a moving mesh's changed occupancy region into the block's texture,
/// as the original's sub-upload does (`glTexSubImage3D` over the dirty box).
fn apply_occupancy_patches(
    patches: Option<Res<GiOccupancyPatches>>,
    textures: Option<Res<GiTextures>>,
    pending: Option<Res<GiPendingTextures>>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
) {
    let Some(patches) = patches else { return };
    if patches.0.is_empty() {
        return;
    }
    let textures = pending.as_ref().map(|p| &p.0).or(textures.as_deref());
    let Some(textures) = textures else { return };
    let Some(occupancy) = images.get(&textures.occupancy) else {
        return;
    };
    for patch in &patches.0 {
        // Only the block whose texture is bound takes the write; a patch for
        // another resident block waits until it is bound.
        if patch.sector != textures.sector {
            continue;
        }
        let size = patch.hi.saturating_sub(patch.lo);
        if size.min_element() == 0 || patch.bytes.is_empty() {
            continue;
        }
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &occupancy.texture,
                mip_level: 0,
                origin: Origin3d {
                    x: patch.lo.x,
                    y: patch.lo.y,
                    z: patch.lo.z,
                },
                aspect: Default::default(),
            },
            &patch.bytes,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.x),
                rows_per_image: Some(size.y),
            },
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: size.z,
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_bind_group(
    mut commands: Commands,
    textures: Option<Res<GiTextures>>,
    pending: Option<Res<GiPendingTextures>>,
    lights: Res<GiLights>,
    sun_shadow: Option<Res<GiSunShadow>>,
    sun_sky: Option<Res<GiSunSky>>,
    shadow_views: Query<
        &bevy::pbr::ViewShadowBindings,
        (
            Without<crate::supersample::DisplayCamera>,
            Without<crate::supersample::Offscreen>,
        ),
    >,
    pipeline: Option<Res<GiPipeline>>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut owed: ResMut<GiOwed>,
    mut buffer: Local<Option<UniformBuffer<GiParams>>>,
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
    let cube = images.get(&crate::cube_shadows::CUBE_POOL);
    let sun = shadow_views
        .iter()
        .next()
        .map(|v| &v.directional_light_depth_texture_view);
    let mut entries = [GiLight::default(); MAX_GROUP_LIGHTS];
    for (slot, l) in entries.iter_mut().zip(&lights.lights) {
        *slot = *l;
    }
    let mut cubes = [UVec4::splat(u32::MAX); MAX_GROUP_LIGHTS / 4];
    for (i, &slot) in lights.cube_slots.iter().enumerate() {
        cubes[i / 4][i % 4] = slot;
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
    let sun_shadow = sun_shadow.as_deref().copied().unwrap_or_default();
    let shaded = sun_shadow.enabled && sun.is_some();
    let value = GiParams {
        origin: textures.origin.extend(0.0),
        extent: textures.extent.extend(0.0),
        base: textures.base_ambient.extend(0.0),
        sun_matrix: sun_shadow.clip_from_world,
        sun_flags: UVec4::new(
            shaded as u32,
            sun_sky.as_deref().is_some_and(|s| s.0) as u32,
            textures.top_slice as u32,
            0,
        ),
        groups,
        sector_groups,
        lights: entries,
        cubes,
        voxel_count: UVec4::new(
            lights.voxel_lights.len().min(MAX_VOXEL_LIGHTS) as u32,
            0,
            0,
            0,
        ),
        voxel_lights: voxel_entries,
    };
    // One buffer, rewritten every frame (the parameters are about 42 KiB).
    let params = match &mut *buffer {
        Some(params) => {
            params.set(value);
            params
        }
        none => none.insert(UniformBuffer::from(value)),
    };
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
            cube.map_or(&pipeline.dummy_cube, |c| &c.texture_view),
            &pipeline.cube_sampler,
            sun.unwrap_or(&pipeline.dummy_sun),
            &pipeline.sun_sampler,
        )),
    );
    if owed.serial != textures.serial {
        *owed = GiOwed {
            serial: textures.serial,
            total: textures.warm_up,
            left: textures.warm_up,
        };
    }
    commands.insert_resource(GiBindGroup { bind_group });
}

#[allow(clippy::too_many_arguments)]
fn propagate(
    view: ViewQuery<(
        &ExtractedView,
        Has<crate::supersample::DisplayCamera>,
        Has<crate::supersample::Offscreen>,
    )>,
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
    // Once per frame, for the scene view (not a supersampling display or an
    // off-screen scene).
    let (_, display, offscreen) = view.into_inner();
    if display || offscreen {
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
    /// The GI parameters must be laid out as a uniform buffer allows: an
    /// array of 4-byte values (a matrix written as `[[f32; 4]; 4]`) has a
    /// stride of 4 and panics at the first upload.
    #[test]
    fn the_gi_parameters_are_a_valid_uniform() {
        use bevy::render::render_resource::ShaderType;
        GiParams::assert_uniform_compat();
    }

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
            top_slice: false,
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
            top_slice: false,
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
    fn the_gi_emissive_base_injects_nothing() {
        assert!(!NoGiShape.injects());
        // Any shape can opt in without the manager knowing its type.
        struct Glow;
        impl GiEmissive for Glow {
            fn injects(&self) -> bool {
                true
            }
        }
        assert!(Glow.injects());
    }

    #[test]
    fn the_presets_follow_the_voxel_size_and_level_height() {
        assert_eq!(
            preset(5, 100),
            GiPreset {
                width: 32,
                height: 32,
                depth: 64,
                voxel_cm: 5
            }
        );
        assert_eq!(preset(20, 1280).depth, 64);
        assert_eq!(preset(20, 1300).depth, 128);
        assert_eq!(preset(40, 100).voxel_cm, 40);
        assert_eq!(preset(40, 100).width, 128);
        assert_eq!(preset(40, 100).depth, 16);
    }

    #[test]
    fn the_corner_offsets_run_x_then_z_then_height() {
        let offsets = corner_offsets(UVec3::new(256, 256, 64));
        let step = Vec3::new(1.33 / 256.0, 1.33 / 256.0, 1.33 / 64.0);
        assert_eq!(offsets[0], [-step.x, -step.y, -step.z, 0.125]);
        assert_eq!(offsets[1], [step.x, -step.y, -step.z, 0.125]);
        assert_eq!(offsets[2], [-step.x, step.y, -step.z, 0.125]);
        assert_eq!(offsets[4], [-step.x, -step.y, step.z, 0.125]);
        assert_eq!(offsets[7], [step.x, step.y, step.z, 0.125]);
        // Each axis is formed in double precision and rounded once to f32,
        // as the original's CVTSI2SD/MULSD/DIVSD/CVTSD2SS chain does.
        let exact = |corner: i32, n: u32| ((f64::from(corner) * 1.33) / f64::from(n)) as f32;
        assert_eq!(offsets[7][0], exact(1, 256));
        assert_eq!(offsets[7][2], exact(1, 64));
        assert_eq!(offsets[0][2], exact(-1, 64));
        // The f32 product would differ on this size, so the double path
        // is not interchangeable with 1.33 / n as f32.
        let f32_step = 1.33f32 / 37.0;
        assert_ne!(exact(1, 37), f32_step);
    }

    #[test]
    fn the_gi_programs_keep_the_originals_bindings() {
        let inject = GiPass::Inject.layout();
        assert_eq!(inject.vertex, "gi_inject_v");
        assert_eq!(inject.geometry, "gi_inject_g");
        assert_eq!(inject.fragment, "gi_inject_f");
        assert_eq!(inject.uniforms[0], "MPrj");
        assert_eq!(inject.uniforms[1], "First");
        assert_eq!(inject.uniforms[2], "Light");
        assert_eq!(inject.uniforms[3], "VSmp");
        assert_eq!(inject.uniforms[4], "ZTrn");
        assert_eq!(inject.uniforms[5], "LyrOfs");
        assert_eq!(inject.uniforms[6], "BaseAmb");
        assert_eq!(inject.uniforms[7], "WrlMat");
        assert_eq!(inject.uniforms[8], "SdwMat");
        assert_eq!(inject.uniforms[9], "SdwMat2");
        assert_eq!(inject.uniforms[10], "SliceSz");
        // Light i binds Shadow<i> to unit 2 + i, then LVox 0 and OVox 1.
        assert_eq!(inject.samplers[0], ("Shadow0", 2));
        assert_eq!(inject.samplers[11], ("Shadow11", 13));
        assert_eq!(inject.samplers[12], ("LVox", 0));
        assert_eq!(inject.samplers[13], ("OVox", 1));
        assert_eq!(shadow_sampler(0), ("Shadow0".into(), 2));
        assert_eq!(shadow_sampler(9), ("Shadow9".into(), 11));
        assert_eq!(shadow_sampler(10), ("Shadow10".into(), 12));
        assert_eq!(shadow_sampler(11), ("Shadow11".into(), 13));
        let emission = GiPass::Emission.layout();
        assert_eq!(emission.fragment, "gi_emission");
        // The emission builder sets only OVox; gi_emission reads no LVox.
        assert_eq!(emission.samplers, &[("OVox", 1)]);
        assert_eq!(
            emission.uniforms,
            &["MPrj", "SliceSz", "ZTrn", "First", "Light"]
        );
        let blit = GiPass::Blit.layout();
        assert_eq!(blit.fragment, "gi_blit_f");
        assert_eq!(blit.samplers, &[("LVox", 0)]);
        assert_eq!(blit.uniforms, &["MPrj", "First", "UVOfs", "SliceSz"]);
    }

    #[test]
    fn the_shared_quad_indices_are_two_triangles_each() {
        assert_eq!(quad_indices(2), vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }

    #[test]
    fn a_shapes_box_is_rounded_and_margin_clipped() {
        let draw = shape_draw(
            ShapeVolume {
                position: Vec3::new(10.0, 2.0, 6.0),
                half: Vec3::splat(1.0),
            },
            Vec3::ZERO,
            BLOCK,
            0.2,
        )
        .unwrap();
        assert_eq!((draw.lo, draw.hi), (UVec2::new(45, 25), UVec2::new(55, 35)));
        assert_eq!((draw.first, draw.count), (5, 11));
        // A wide shape is clipped to the 8-voxel margins.
        let draw = shape_draw(
            ShapeVolume {
                position: Vec3::new(0.0, 2.0, 0.0),
                half: Vec3::splat(4.0),
            },
            Vec3::ZERO,
            BLOCK,
            0.2,
        )
        .unwrap();
        assert_eq!((draw.lo, draw.hi), (UVec2::new(8, 8), UVec2::new(20, 20)));
        // A centre below slice 2 draws nothing; slice 2 draws. Half extents
        // round up: 0.3 m is 1.5 voxels, so 2.
        let at = |y: f32| {
            shape_draw(
                ShapeVolume {
                    position: Vec3::new(10.0, y, 6.0),
                    half: Vec3::splat(0.3),
                },
                Vec3::ZERO,
                BLOCK,
                0.2,
            )
        };
        assert!(at(-1.0).is_none());
        assert!(at(0.2).is_none());
        let low = at(0.4).unwrap();
        assert_eq!((low.first, low.count), (0, 5));
        assert_eq!((low.lo, low.hi), (UVec2::new(48, 28), UVec2::new(52, 32)));
    }

    #[test]
    fn the_manager_copies_a_neighbours_margins() {
        let dims = UVec3::new(256, 256, 64);
        assert!(margin_blit(dims, 0, 0).is_none());
        let left = margin_blit(dims, -1, 0).unwrap();
        assert_eq!((left.lo, left.hi), (UVec2::new(0, 8), UVec2::new(8, 248)));
        assert_eq!(left.offset, Vec2::new(0.9375, 0.0));
        assert_eq!(left.slices, 63);
        let right = margin_blit(dims, 1, 0).unwrap();
        assert_eq!(
            (right.lo, right.hi),
            (UVec2::new(248, 8), UVec2::new(256, 248))
        );
        assert_eq!(right.offset, Vec2::new(-0.9375, 0.0));
        let corner = margin_blit(dims, 1, 1).unwrap();
        assert_eq!(
            (corner.lo, corner.hi),
            (UVec2::new(248, 248), UVec2::new(256, 256))
        );
        assert_eq!(corner.offset, Vec2::splat(-0.9375));
        // The blit's quad lies on the destination strip with the original's
        // base z of −0.5/64; its fragment adds the UV offset to read the
        // neighbour.
        let quad = margin_quad(&left, dims);
        assert_eq!(quad[0], Vec3::new(0.0, 8.0 / 256.0, -0.5 / 64.0));
        assert_eq!(quad[1], Vec3::new(8.0 / 256.0, 8.0 / 256.0, -0.5 / 64.0));
        assert_eq!(quad[2], Vec3::new(8.0 / 256.0, 248.0 / 256.0, -0.5 / 64.0));
        assert_eq!(quad[3], Vec3::new(0.0, 248.0 / 256.0, -0.5 / 64.0));
    }

    #[test]
    fn the_sun_is_the_first_light_of_every_group() {
        let mut sectors = [GiSectorPlan::default(); 64];
        sectors[0] = plan(true, 10, &[SUN_LIGHT_ID, 5]);
        sectors[1] = plan(true, 10, &[SUN_LIGHT_ID, 5]);
        let (groups, _) = group(&sectors, 64);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights.ids(), &[SUN_LIGHT_ID, 5]);
        // The sun counts towards the twelve-light cap: eleven point lights
        // plus it fill a group.
        let eleven: Vec<u32> = (0..11).collect();
        let mut with_sun = vec![SUN_LIGHT_ID];
        with_sun.extend(eleven);
        let mut sectors = [GiSectorPlan::default(); 64];
        sectors[0] = plan(true, 10, &with_sun);
        sectors[1] = plan(true, 10, &with_sun);
        let (groups, _) = group(&sectors, 64);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights.ids().len(), 12);
    }

    #[test]
    fn light_uniforms_use_the_blocks_texture_units() {
        let mut sources = HashMap::new();
        sources.insert(
            1,
            LightSource {
                colour: Vec3::ONE,
                intensity: 1.0,
                radius: 2.0,
                position: Vec3::new(10.0, 2.0, 6.0),
                multiplier: 1.0,
                shaping: Vec2::ONE,
                dim: 0.0,
                bias: 0.025,
                ranged: true,
                cube: 3,
                sun: false,
            },
        );
        let mut groups = vec![GiGroupPlan {
            floor: 0,
            lights: GiSector::new([1]),
            sectors: 1,
        }];
        let mut gi = GiLights::default();
        push_groups(
            &mut gi,
            &groups,
            &sources,
            [0; 64],
            Vec3::new(0.0, -6.8, 0.0),
            // World axes: 51.2 m across, 12.8 m tall.
            Vec3::new(51.2, 12.8, 51.2),
        );
        let light = gi.lights[0];
        let expected = Vec4::new(10.0 / 51.2, 6.0 / 51.2, 8.8 / 51.2, 25.6);
        assert!((light.position_radius - expected).abs().max_element() < 1e-6);
        assert!((light.colour_shadow.x - 0.15).abs() < 1e-6);
        // ((0.025 bias + 0.94) / 200 cm) × 5120, in double.
        let scale = ((0.025f64 + 0.94) / 200.0 * 5120.0) as f32;
        assert_eq!(light.colour_shadow.w, scale);
        assert_eq!(light.shaping, Vec4::new(1.0, 1.0, 1.0, 0.0));
        assert_eq!(gi.cube_slots, vec![2]);
        // The sun's colour is scaled by 3 × 0.3, not 0.15.
        sources.insert(
            SUN_LIGHT_ID,
            LightSource {
                sun: true,
                cube: 0,
                ..sources[&1]
            },
        );
        let mut gi = GiLights::default();
        groups[0].lights = GiSector::new([SUN_LIGHT_ID]);
        push_groups(&mut gi, &groups, &sources, [0; 64], Vec3::ZERO, Vec3::ONE);
        assert!((gi.lights[0].colour_shadow.x - 0.9).abs() < 1e-6);
        assert_eq!(gi.lights[0].shaping.w, 1.0);
        assert_eq!(gi.cube_slots, vec![u32::MAX]);
        // The original's separate GI sun map's numbers.
        assert_eq!(GI_SUN_SHADOW_SIZE, 1024);
        assert_eq!(GI_SUN_SOFTNESS, 1600.0);
    }

    #[test]
    fn voxel_light_legacy_path_uses_six_tenths_in_double() {
        // √16 × 0.6 = 2.4, where the modern path gives × 3 = 12.
        let legacy = VoxelLight::colour_for([1.0; 3], 16.0, true);
        let modern = VoxelLight::colour_for([1.0; 3], 16.0, false);
        assert!((legacy.x - 2.4).abs() < 1e-6);
        assert!((modern.x - 12.0).abs() < 1e-6);
        // √2 × 0.6 is worked out as a double and rounded once.
        let root_two = (f64::from(2f32.sqrt()) * 0.6) as f32;
        assert_eq!(VoxelLight::colour_for([1.0; 3], 2.0, true).x, root_two);
    }

    #[test]
    fn voxel_light_activation_copies_its_base_radius_and_flags() {
        let mut light = VoxelLight::from_record([1.0, 0.5, 0.25], 4.0, 3.5);
        assert_eq!(light.radius, 3.5);
        assert_eq!(light.active_radius, 3.5);
        assert_eq!(light.base_intensity, 4.0);
        assert_eq!(light.flags, voxel_flags::ACTIVE | voxel_flags::COLOUR_DIRTY);
        // A new colour update marks the colour but leaves the light active.
        light.flags = 0;
        light.colour_update(false);
        assert_eq!(light.flags, voxel_flags::COLOUR_DIRTY);
        // A run-time intensity change recomputes the colour: √9 × 3.
        light.set_intensity(9.0);
        assert_eq!(light.intensity, 9.0);
        assert_eq!(light.base_intensity, 4.0);
        assert!((light.colour.z - (9f32.sqrt() * 3.0) * linear_rgb([0.25; 3]).z).abs() < 1e-6);
        // Activation after a radius change copies the new base radius.
        light.radius = 7.0;
        light.activate();
        assert_eq!(light.active_radius, 7.0);
        assert_eq!(light.flags, voxel_flags::ACTIVE | voxel_flags::COLOUR_DIRTY);
    }

    #[test]
    fn voxel_light_clone_takes_the_master_values() {
        let mut master = VoxelLight::from_record([0.2, 0.4, 1.0], 16.0, 2.0);
        master.flags = 0xFFFF;
        let copy = master.copied_from();
        assert_eq!(copy.color, master.color);
        assert_eq!(copy.intensity, 16.0);
        assert_eq!(copy.base_intensity, 16.0);
        assert_eq!(copy.radius, 2.0);
        assert_eq!(copy.active_radius, 2.0);
        assert_eq!(copy.colour, master.colour);
        // The clone's flag word is its own, not the master's.
        assert_eq!(copy.flags, 0);
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

    fn group(sectors: &[GiSectorPlan; 64], depth: u32) -> (Vec<GiGroupPlan>, [u8; 64]) {
        let mut groups = Vec::new();
        let of_sector = group_sectors(sectors, depth, &mut groups);
        (groups, of_sector)
    }

    #[test]
    fn the_light_share_drops_at_the_high_shading_quality() {
        assert_eq!(injection_share(false), 0.1);
        // The product is single precision: 0.1 × 0.9 rounds to 0.089999996,
        // one step below the decimal 0.09's nearest float.
        assert_eq!(injection_share(true), 0.1f32 * 0.9);
        assert_eq!(injection_share(true).to_bits(), 0x3db8_51eb);
        assert!(injection_share(true) < injection_share(false));
    }

    #[test]
    fn the_fill_estimate_counts_lights_sectors_and_slices() {
        // (2 + 2) lights × 3 sectors × (63 − 46) slices + 80.
        assert_eq!(GiGroupPlan::cost(2, 3, 46, 64), 284);
        // A group at the block's top draws no slice: only the 80.
        assert_eq!(GiGroupPlan::cost(5, 4, 63, 64), 80);
    }

    #[test]
    fn sectors_with_the_same_lights_share_a_group_and_its_lowest_floor() {
        let mut sectors = [GiSectorPlan::default(); 64];
        sectors[0] = plan(true, 46, &[3, 7]);
        sectors[1] = plan(true, 48, &[3, 7]);
        // Three slices from the group's floor: a group of its own.
        sectors[2] = plan(true, 49, &[3, 7]);
        // An unmarked sector, or one without geometry, is left out.
        sectors[3] = plan(false, 46, &[3, 7]);
        sectors[4] = plan(true, u8::MAX, &[3, 7]);
        // The same lights listed in another order are the same set.
        sectors[5] = plan(true, 47, &[7, 3]);
        let mut groups = Vec::new();
        let mut first = sectors;
        first[2].drawn = false;
        group_sectors(&first, 64, &mut groups);
        assert_eq!(groups.len(), 1);
        assert_eq!((groups[0].floor, groups[0].sectors), (46, 3));
        // The merge pass then folds the second group in: apart they cost
        // 4·3·17 + 80 = 284 and 4·1·14 + 80 = 136, together
        // 4·4·17 + 80 = 352, below 420.
        let (groups, of_sector) = group(&sectors, 64);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights.ids(), &[3, 7]);
        assert_eq!((groups[0].floor, groups[0].sectors), (46, 4));
        assert_eq!(of_sector[..6], [0, 0, 0, u8::MAX, u8::MAX, 0]);
    }

    #[test]
    fn groups_merge_only_when_drawing_them_together_costs_less() {
        let mut sectors = [GiSectorPlan::default(); 64];
        // Apart 3·117 + 80 = 431 and 3·27 + 80 = 161; together
        // 3·2·117 + 80 = 782: kept apart.
        sectors[0] = plan(true, 10, &[1]);
        sectors[1] = plan(true, 100, &[1]);
        let (groups, _) = group(&sectors, 128);
        assert_eq!(groups.len(), 2);
        // Floors 10 and 14 start apart (four slices) but together cost
        // 782, below 431 + 419: merged, at the lower floor.
        sectors[1] = plan(true, 14, &[1]);
        let (groups, of_sector) = group(&sectors, 128);
        assert_eq!(groups.len(), 1);
        assert_eq!((groups[0].floor, groups[0].sectors), (10, 2));
        assert_eq!(of_sector[..2], [0, 0]);
    }

    #[test]
    fn groups_merge_only_where_their_lights_overlap_and_stay_within_twelve() {
        let mut sectors = [GiSectorPlan::default(); 64];
        // [1, 2] and [2, 5] share 2. With 53 slices the third light costs
        // more than a second draw (610 against 292 + 292).
        sectors[0] = plan(true, 10, &[1, 2]);
        sectors[1] = plan(true, 10, &[2, 5]);
        assert_eq!(group(&sectors, 64).0.len(), 2);
        // With 33 it does not (410 against 212 + 212): the second group's
        // new light joins the first's list.
        sectors[0] = plan(true, 30, &[1, 2]);
        sectors[1] = plan(true, 30, &[2, 5]);
        let (groups, of_sector) = group(&sectors, 64);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights.ids(), &[1, 2, 5]);
        assert_eq!(of_sector[..2], [0, 0]);

        // Disjoint light sets never merge.
        let mut sectors = [GiSectorPlan::default(); 64];
        sectors[0] = plan(true, 60, &[1]);
        sectors[1] = plan(true, 60, &[2]);
        assert_eq!(group(&sectors, 64).0.len(), 2);
        // Two empty sets do: apart 2·53 + 80 and 2·43 + 80, together
        // 2·2·53 + 80 = 292 < 352.
        sectors[0] = plan(true, 10, &[]);
        sectors[1] = plan(true, 20, &[]);
        let (groups, _) = group(&sectors, 64);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].floor, 10);

        // Eight lights and six sharing one make thirteen, over the cap.
        let eight: Vec<u32> = (0..8).collect();
        let six: Vec<u32> = (7..13).collect();
        sectors[0] = plan(true, 60, &eight);
        sectors[1] = plan(true, 60, &six);
        assert_eq!(group(&sectors, 64).0.len(), 2);
        // Five sharing one with eight give twelve; with three slices the
        // merge costs 14·2·3 + 80 = 164 against 110 + 101.
        let five: Vec<u32> = (7..12).collect();
        sectors[1] = plan(true, 60, &five);
        let (groups, _) = group(&sectors, 64);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].lights.ids().len(), 12);
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
            dim: 0.0,
            bias: 0.025,
            ranged: true,
            cube: 0,
            sun: false,
        };
        // The cell's centre at the origin, the floor slice's middle 1 m up
        // (in centimetres).
        let (cell, floor_y) = (Vec2::ZERO, 100.0);
        let reaches = |at: Vec3, y: f32| shape_reaches(&source(at, y), cell, floor_y);
        // Straight above passes, and so does a light within half a cell.
        assert!(reaches(Vec3::new(0.0, 5.0, 0.0), 0.0));
        assert!(reaches(Vec3::new(2.9, 5.0, 0.0), 0.0));
        // At the slice's height (no direction at all) or below it, a
        // shaped light is cut.
        assert!(!reaches(Vec3::new(0.0, 1.0, 0.0), 0.0));
        assert!(!reaches(Vec3::new(0.0, -3.0, 0.0), 0.0));
        // A 5 m offset is pulled in to 2 m; under a 5 m drop it carries.
        assert!(reaches(Vec3::new(5.0, 6.0, 0.0), 0.0));
        // A negative shaping cuts a shallow drop: c = 50 / √42 500 and
        // c − (1 − c) < 0.
        assert!(!reaches(Vec3::new(5.0, 1.5, 0.0), -1.0));
        assert!(reaches(Vec3::new(0.0, 5.0, 0.0), -1.0));
        // An offset that is not a number counts as none.
        assert!(reaches(Vec3::new(f32::NAN, 5.0, 0.0), 0.0));
        // A light without a spot always passes.
        assert!(reaches(Vec3::new(5.0, -20.0, 0.0), 1.0));
        assert_eq!(SHAPE_FLOOR.to_bits(), 0x33ff_ffd7);
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

    #[test]
    fn a_sector_reads_its_30_columns() {
        let mut t = fake_textures(1);
        t.dims = BLOCK;
        let mut floors = vec![255u8; 256 * 256];
        for z in 8 + 30 * 5..8 + 30 * 6 {
            for x in 8 + 30 * 3..8 + 30 * 4 {
                floors[x + z * 256] = 12;
            }
        }
        t.floors = Arc::new(floors);
        assert_eq!(t.sector_floor(3, 5), 12);
        assert_eq!(t.sector_floor(2, 5), 255);
        assert_eq!(t.sector_floor(3, 4), 255);
        assert_eq!(t.sector_floor(8, 0), 255);
    }

    #[test]
    fn the_top_slice_is_drawn_only_under_the_level_flag() {
        assert_eq!(drawn_slices(3, 64, false), 3..63);
        assert_eq!(drawn_slices(3, 64, true), 3..64);
        assert_eq!(drawn_slices(0, 128, true), 0..128);
        // A floor at or above the top draws nothing.
        assert!(drawn_slices(70, 64, true).is_empty());
        assert!(drawn_slices(63, 64, false).is_empty());
    }

    #[test]
    fn the_injection_reads_each_voxels_own_slice_centre() {
        // The vertex shader's quad r is −0.5/depth and the layer offset is
        // subtracted, so layer 0 reads r = 0.5/64, not 0.
        let dims = UVec3::new(256, 256, 64);
        assert_eq!(
            voxel_uv(UVec3::ZERO, dims),
            Vec3::new(0.5 / 256.0, 0.5 / 256.0, 0.5 / 64.0)
        );
        assert_eq!(
            voxel_uv(UVec3::new(255, 255, 63), dims),
            Vec3::new(255.5 / 256.0, 255.5 / 256.0, 63.5 / 64.0)
        );
        // The light distances use the height over the width, a slice's
        // middle sitting at (slice + 0.5)/256.
        assert_eq!(
            voxel_height(UVec3::new(0, 0, 46), dims),
            Vec3::new(0.5 / 256.0, 0.5 / 256.0, 46.5 / 256.0)
        );
    }

    #[test]
    fn the_gi_shadow_bias_follows_the_shadow_detail() {
        // The shipped detail 5: 0.025 at offsets 0 and +1 (capped at 6).
        assert_eq!(gi_shadow_bias(0), 0.025);
        assert_eq!(gi_shadow_bias(1), 0.025);
        // A lower offset widens the shadow: q = 4, 3.
        assert_eq!(gi_shadow_bias(-1), 0.02);
        assert_eq!(gi_shadow_bias(-2), 0.015);
        assert_eq!(GI_SHADOW_DETAIL, 5);
        assert_eq!(GiShadowBias::default(), GiShadowBias(0.025));
    }

    #[test]
    fn the_gi_cube_lookup_scales_the_distance_by_bias_and_radius() {
        let source = |bias: f32, radius: f32, ranged: bool| LightSource {
            colour: Vec3::ONE,
            intensity: 1.0,
            radius,
            position: Vec3::ZERO,
            multiplier: 1.0,
            shaping: Vec2::ONE,
            dim: 0.0,
            bias,
            ranged,
            cube: 1,
            sun: false,
        };
        // The shipped bias 0.025: ((0.025 + 0.94)/200 cm) × 5120 = 24.704.
        let scale = source(0.025, 2.0, true).cube_shadow_scale();
        assert_eq!(scale, ((0.025f64 + 0.94) / 200.0 * 5120.0) as f32);
        assert!((scale - 24.704).abs() < 1e-4);
        // A lower shadow detail widens the bias: q = 3 gives 0.015.
        let scale = source(0.015, 2.0, true).cube_shadow_scale();
        assert_eq!(scale, ((0.015f64 + 0.94) / 200.0 * 5120.0) as f32);
        // An unranged light's far plane is 3.4e38: the compare value stays
        // at the 0.01 bias and every tap is lit.
        let scale = source(0.025, 2.0, false).cube_shadow_scale();
        assert!(scale > 0.0 && scale < 1e-33, "{scale}");
    }

    #[test]
    fn surfaces_read_the_voxels_centre_and_terrain_its_bottom() {
        let mut t = fake_textures(1);
        t.origin = Vec3::new(1.0, -6.8, 2.0);
        t.voxel = 0.2;
        // Half a slice up: the original's per-object `(below + 0.5)/depth`.
        assert_eq!(t.sample_origin(), Vec4::new(1.0, -6.9, 2.0, 1.0));
        // Terrain and grass pass the flag that drops it.
        assert_eq!(t.sample_origin_floor(), Vec4::new(1.0, -6.8, 2.0, 1.0));
    }

    #[test]
    fn a_shapes_quad_walks_its_clipped_box() {
        let dims = UVec3::new(256, 256, 64);
        let draw = shape_draw(
            ShapeVolume {
                position: Vec3::new(10.0, 2.0, 6.0),
                half: Vec3::splat(1.0),
            },
            Vec3::ZERO,
            BLOCK,
            0.2,
        )
        .unwrap();
        // Columns 45..55 and 25..35, the base slice's centre negated
        // (first 5, so r = −5.5/64) with the fragment flipping it back.
        let quad = shape_quad(&draw, dims);
        assert_eq!(quad[0], Vec3::new(45.0 / 256.0, 25.0 / 256.0, -5.5 / 64.0));
        assert_eq!(quad[1], Vec3::new(55.0 / 256.0, 25.0 / 256.0, -5.5 / 64.0));
        assert_eq!(quad[2], Vec3::new(55.0 / 256.0, 35.0 / 256.0, -5.5 / 64.0));
        assert_eq!(quad[3], Vec3::new(45.0 / 256.0, 35.0 / 256.0, -5.5 / 64.0));
        // The shared uniforms: 1/64 slices and (−64/256, 64/256).
        let (slice, z) = shape_uniforms(dims);
        assert_eq!(slice, 1.0 / 64.0);
        assert_eq!(z, Vec2::new(-0.25, 0.25));
    }

    #[test]
    fn the_propagation_shader_validates() {
        let module =
            naga::front::wgsl::parse_str(include_str!("gi.wesl")).expect("gi.wesl parses as WGSL");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("gi.wesl validates");
        // The whole uniform block fits wgpu's default 64 KiB binding.
        let size = <GiParams as ShaderType>::min_size().get();
        assert!(size > 0 && size <= 64 * 1024, "{size} bytes");
    }

    /// The CPU's uniform structs are exactly as large as the shader's, so
    /// the binding is neither short (a validation error on every GI frame)
    /// nor read at shifted offsets.
    #[test]
    fn the_gi_uniforms_match_the_shaders_layout() {
        let module =
            naga::front::wgsl::parse_str(include_str!("gi.wesl")).expect("gi.wesl parses as WGSL");
        let mut layouter = naga::proc::Layouter::default();
        layouter.update(module.to_ctx()).expect("gi.wesl lays out");
        let size_of = |name: &str| {
            let (handle, _) = module
                .types
                .iter()
                .find(|(_, ty)| ty.name.as_deref() == Some(name))
                .unwrap_or_else(|| panic!("{name} is in gi.wesl"));
            u64::from(layouter[handle].size)
        };
        assert_eq!(size_of("GiLight"), 48);
        assert_eq!(
            size_of("GiLight"),
            <GiLight as ShaderType>::min_size().get()
        );
        assert_eq!(
            size_of("GiVoxelLight"),
            <GiVoxelLight as ShaderType>::min_size().get()
        );
        assert_eq!(
            size_of("GiGroup"),
            <GiGroupUniform as ShaderType>::min_size().get()
        );
        assert_eq!(
            size_of("GiParams"),
            <GiParams as ShaderType>::min_size().get()
        );
    }
}
