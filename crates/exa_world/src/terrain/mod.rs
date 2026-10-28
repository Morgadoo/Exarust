//! Terrain: one heightfield grid per `.rft` tile, drawn as one mesh per 6 m
//! block and textured by layering the palette materials its painted layers
//! resolve to.
//!
//! Layers name a palette material or a blend rule; [`blend`] evaluates rules
//! per vertex into material weights. A tile stacks every material it uses
//! (up to 16) in palette order, like the original: the first is opaque and
//! each later one covers what is below by an "over" opacity, sharpened by
//! the material's blend mask. The opacities travel to the shader in a
//! per-tile splat texture, one texel per vertex. A tile's geometry and masks
//! are built on the async compute pool, as the original's terrain worker
//! does them; [`patch`] is the original's shared triangle list, [`ground`]
//! its world height query, [`stream`] its streaming decisions and tile bits,
//! [`composite`] its per-block texture sets, passes and selection, and
//! [`wireframe`] its debug wireframe overlay.

pub mod manager;

pub mod blend;
pub mod composite;
pub mod frame;
pub mod ground;
pub mod height;
pub mod live;
pub mod patch;
pub mod plugins;
pub mod stream;
pub mod vegetation;
pub mod wireframe;

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use exa_assets::texture::ColorSpace;
use exa_assets::{ModelLoader, PackKind};
use exa_formats::rfi::Rfi;
use exa_formats::terrain::{
    Palette, TILE_VERTICES, Terrain, TerrainMaterial as PaletteMaterial, TerrainTexture,
    TerrainTile,
};

use blend::{NoiseMaps, RuleTree, over_alphas};
use exa_formats::level::LevelGrid;

use crate::room_view::InRoom;
use crate::rooms::{CELL_TERRAIN, Rooms};
use height::{HeightLayer, Plane, displacement, vegetation as vegetation_density};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Materials a tile stacks: every one it uses, as the original stacks them
/// (the shipped tiles use up to 12).
const SPLAT_LAYERS: usize = 16;
/// Opacities per layer of the splat texture (RGBA).
const SPLAT_CHANNELS: usize = 4;
/// Texture coordinates advance this much per metre before a material's
/// tiling factor (4 per 48 m tile).
const UV_PER_METRE: f32 = 4.0 / 48.0;
/// Diffuse colour is texture × tint × this.
const TINT_GAIN: f32 = 1.5;
/// Depth of the skirt below each tile's edge, metres.
const SKIRT_DEPTH: f32 = 0.5;
/// Materials whose share of a tile is below this are dropped.
const MIN_TILE_SHARE: f32 = 1.0 / 510.0;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "terrain.wesl");
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(
                PostUpdate,
                (
                    apply_splats,
                    give_terrain_gi,
                    exa_render::supersample::sync_split_taps::<TerrainMaterial>,
                    sync_plant_shadows,
                )
                    .chain(),
            )
            .add_systems(Update, live::drive_terrain_stream);
    }
}

/// The terrain's material: its layered textures, shaded with the original's
/// lighting (`exa_render::shading`) and its terrain gloss rule.
pub type TerrainMaterial = TerrainSplat;

/// Declares [`TerrainSplat`] with one diffuse, surface and normal texture
/// per layer (`layer => diffuse binding, surface binding, normal binding`).
macro_rules! terrain_splat {
    ($($k:tt => $diffuse:ident $bd:tt, $surface:ident $bs:tt, $normal:ident $bn:tt;)*) => {
        #[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
        pub struct TerrainSplat {
            #[uniform(100)]
            pub params: SplatParams,
            /// The layers' shared repeating sampler, carried by a 1 × 1
            /// stand-in image (the derive binds the image too).
            #[texture(101)]
            #[sampler(102)]
            pub layer_sampler: Option<Handle<Image>>,
            /// Each vertex's over opacities: four RGBA layers of 241 × 241,
            /// layer k in channel k % 4 of layer k / 4.
            #[texture(103, dimension = "2d_array")]
            #[sampler(104)]
            pub splat: Option<Handle<Image>>,
            $(
                #[texture($bd)]
                pub $diffuse: Option<Handle<Image>>,
                /// Parallax height (red), gloss (green) and blend mask
                /// (blue) packed together.
                #[texture($bs)]
                pub $surface: Option<Handle<Image>>,
                #[texture($bn)]
                pub $normal: Option<Handle<Image>>,
            )*
            /// The voxel GI light volume, once it exists.
            #[texture(123, dimension = "3d")]
            #[sampler(124)]
            pub gi_volume: Option<Handle<Image>>,
            /// The environment cube.
            #[texture(125, dimension = "cube")]
            #[sampler(126)]
            pub environment: Option<Handle<Image>>,
            /// Light records ([`exa_render::lights`]); the default handle is
            /// the shared buffer.
            #[storage(130, read_only)]
            pub lights: Handle<bevy::render::storage::ShaderBuffer>,
            /// The cached cube shadow pool (`exa_render::cube_shadows`).
            #[texture(131, dimension = "cube_array", sample_type = "depth")]
            #[sampler(132, sampler_type = "comparison")]
            pub shadow_cubes: Handle<Image>,
        }

        impl TerrainSplat {
            /// Layer `k`'s diffuse, surface and normal textures.
            fn layer_mut(&mut self, k: usize) -> Option<[&mut Option<Handle<Image>>; 3]> {
                match k {
                    $($k => Some([&mut self.$diffuse, &mut self.$surface, &mut self.$normal]),)*
                    _ => None,
                }
            }
        }

        impl Default for TerrainSplat {
            fn default() -> Self {
                Self {
                    params: SplatParams::default(),
                    layer_sampler: None,
                    splat: None,
                    $($diffuse: None, $surface: None, $normal: None,)*
                    gi_volume: None,
                    environment: None,
                    lights: Handle::default(),
                    shadow_cubes: exa_render::cube_shadows::CUBE_POOL,
                }
            }
        }
    };
}

terrain_splat! {
    0 => diffuse0 140, surface0 160, normal0 180;
    1 => diffuse1 141, surface1 161, normal1 181;
    2 => diffuse2 142, surface2 162, normal2 182;
    3 => diffuse3 143, surface3 163, normal3 183;
    4 => diffuse4 144, surface4 164, normal4 184;
    5 => diffuse5 145, surface5 165, normal5 185;
    6 => diffuse6 146, surface6 166, normal6 186;
    7 => diffuse7 147, surface7 167, normal7 187;
    8 => diffuse8 148, surface8 168, normal8 188;
    9 => diffuse9 149, surface9 169, normal9 189;
    10 => diffuse10 150, surface10 170, normal10 190;
    11 => diffuse11 151, surface11 171, normal11 191;
    12 => diffuse12 152, surface12 172, normal12 192;
    13 => diffuse13 153, surface13 173, normal13 193;
    14 => diffuse14 154, surface14 174, normal14 194;
    15 => diffuse15 155, surface15 175, normal15 195;
}

#[derive(ShaderType, Reflect, Debug, Clone, Copy, Default)]
pub struct SplatParams {
    /// Linear RGB multiplier per material; alpha is its cover strength.
    pub colour: [Vec4; SPLAT_LAYERS],
    /// Mask sharpness `s`, `t`, 1 when the layer has a blend mask, then its
    /// texture repeats per metre.
    pub blend: [Vec4; SPLAT_LAYERS],
    /// Mask sharpness `s`, `t` of the normal pass, normal-map strength (0
    /// without a normal map), gloss.
    pub normal: [Vec4; SPLAT_LAYERS],
    /// Per layer: gloss factor (f10 × 1.5 × min(f9, 1)), then the parallax
    /// height's base, reference and scale (f20 + (B − f21)·f22).
    pub surface: [Vec4; SPLAT_LAYERS],
    /// The GI volume's lowest corner and size (metres).
    pub gi_origin: Vec4,
    pub gi_extent: Vec4,
    /// The palette's wet-floor source ([`exa_formats::terrain::Palette::wetness`]).
    /// Non-zero only turns the wet-floor flag on; the shader reads no value.
    pub wetness: f32,
    /// [`terrain_flags`].
    pub flags: u32,
    /// Layers in use.
    pub layers: u32,
    pub pad1: u32,
    pub pad2: u32,
    pub pad3: u32,
}

/// Which optional inputs a terrain material has.
pub mod terrain_flags {
    pub const GI: u32 = 1;
    pub const ENVIRONMENT: u32 = 2;
    /// Debug view: the raw GI sample (`EXA_GI_DEBUG`).
    pub const DEBUG_GI: u32 = 4;
    /// The frame is supersampled: each sub-frame takes its share of the
    /// shadow and GI taps (`exa_render::supersample::SplitTaps`).
    pub const SPLIT_TAPS: u32 = 8;
    /// The palette's wet-floor source is above 0.4 (the sewer): the lit
    /// program's wet-floor gloss term applies.
    pub const WET_FLOOR: u32 = 16;
}

impl exa_render::supersample::SplitTapsFlag for TerrainMaterial {
    const BIT: u32 = terrain_flags::SPLIT_TAPS;
    fn flags(&self) -> u32 {
        self.params.flags
    }
    fn flags_mut(&mut self) -> &mut u32 {
        &mut self.params.flags
    }
}

fn shader() -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(embedded_path!("terrain.wesl")).with_source("embedded"),
    )
}

impl Material for TerrainSplat {
    fn fragment_shader() -> ShaderRef {
        shader()
    }
}

/// A tile waiting for its material, added by [`apply_splats`] which has
/// access to the material assets: to the tile's mesh, or to its
/// [`TerrainPart`]s in a grid level.
#[derive(Component)]
pub struct PendingSplat(pub TerrainSplat);

/// One room's share of a terrain tile in a grid level, under the tile's
/// entity.
#[derive(Component)]
pub struct TerrainPart;

/// One 6 m terrain block: the original's `TTerrainMesh`, a mesh node that
/// draws a 30 × 30 cell window of the tile's shared patch. The block's
/// frame is the flat 10 cm slab its bounds report:
///
/// `step = size / 2^level`, `half = size / 2^(level+1)` and
/// `first = half − size/2`; block `(bx, by)` sits at
/// `(step·bx + first, 0, step·by + first)` in the tile's space, half
/// extents `(half, 10 cm, half)` and radius the box's half diagonal. At the
/// original's 48 m tile and level 3 (8 × 8 blocks) the step is 6 m, half 3,
/// first −21.
///
/// Activation sets the node visible, gives it the floor's shadow-bias
/// level (3) and its light-link distances 1, then builds its kd-tree;
/// Exarust's room visibility is the activation, and its shadow bias is the
/// same floor level.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct TerrainBlock {
    pub bx: u32,
    pub by: u32,
    /// The block level: 3 for the original's eight blocks a side.
    pub level: u32,
    pub room: Option<u16>,
}

impl TerrainBlock {
    /// The original's block level (eight 6 m blocks across a 48 m tile).
    pub const LEVEL: u32 = 3;
    /// The slab's half height, metres (the original's 10 cm).
    pub const HALF_HEIGHT: f32 = 0.1;

    /// The step between block origins and the half extent for a tile of
    /// `size` metres.
    pub fn step(size: f32, level: u32) -> (f32, f32) {
        let half = size / (2u32 << level) as f32;
        (size / (1u32 << level) as f32, half)
    }

    /// The block's centre in its tile's space, metres.
    pub fn centre_in_tile(size: f32, bx: u32, by: u32, level: u32) -> Vec3 {
        let (step, half) = Self::step(size, level);
        let first = half - size * 0.5;
        Vec3::new(step * bx as f32 + first, 0.0, step * by as f32 + first)
    }

    /// The block's half extents, metres.
    pub fn half_extents(size: f32, level: u32) -> Vec3 {
        let (_, half) = Self::step(size, level);
        Vec3::new(half, Self::HALF_HEIGHT, half)
    }

    /// The block's bounding radius: the half extents' length.
    pub fn radius(size: f32, level: u32) -> f32 {
        Self::half_extents(size, level).length()
    }

    /// Its bounds, for the collision and shadow links.
    pub fn bounds(&self, size: f32, tile_centre: Vec3) -> crate::PartBounds {
        crate::PartBounds {
            centre: Self::centre_in_tile(size, self.bx, self.by, self.level) + tile_centre,
            radius: Self::radius(size, self.level),
            half_extents: Self::half_extents(size, self.level),
        }
    }

    /// The shadow-bias level activation gives the block.
    pub fn shadow_bias(&self) -> u32 {
        exa_render::shadow_bias::FLOOR
    }
}

fn apply_splats(
    mut commands: Commands,
    pending: Query<(Entity, &PendingSplat, Option<&Children>, Has<Mesh3d>)>,
    parts: Query<(), With<TerrainPart>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    for (e, splat, children, mesh) in &pending {
        let material = materials.add(splat.0.clone());
        if mesh {
            commands.entity(e).insert(MeshMaterial3d(material.clone()));
        }
        for &part in children.into_iter().flatten() {
            if parts.contains(part) {
                commands
                    .entity(part)
                    .insert(MeshMaterial3d(material.clone()));
            }
        }
        commands.entity(e).remove::<PendingSplat>();
    }
}

/// Points terrain materials at the GI volume when it is (re)built, and new
/// materials as they come.
fn give_terrain_gi(
    textures: Option<Res<exa_render::gi::GiTextures>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut events: MessageReader<AssetEvent<TerrainMaterial>>,
) {
    let Some(textures) = textures else {
        events.clear();
        return;
    };
    let added: Vec<AssetId<TerrainMaterial>> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Added { id } => Some(*id),
            _ => None,
        })
        .collect();
    let ids: Vec<AssetId<TerrainMaterial>> = if textures.is_changed() {
        materials.ids().collect()
    } else {
        added
    };
    for id in ids {
        let Some(mut m) = materials.get_mut(id) else {
            continue;
        };
        m.gi_volume = Some(textures.light.clone());
        // The terrain and grass callers read the voxel's bottom, not its
        // centre (the original passes the flag that drops the half slice).
        m.params.gi_origin = textures.sample_origin_floor();
        m.params.gi_extent = textures.sample_extent();
        m.params.flags |= terrain_flags::GI;
        if std::env::var_os("EXA_GI_DEBUG").is_some() {
            m.params.flags |= terrain_flags::DEBUG_GI;
        }
    }
}

/// What a terrain spawn produced.
#[derive(Debug, Clone, Default)]
pub struct TerrainStats {
    pub tiles: usize,
    /// Painted layers naming nothing in the palette (their weight falls to
    /// the first rule, as in the original).
    pub unresolved_layers: usize,
    /// Tiles that use more materials than the four a tile can show.
    pub truncated_tiles: usize,
    /// Tiles whose painted brushes passed the original's eleven-brush limit.
    pub brush_limit_tiles: usize,
    /// Grass tufts and plants grown.
    pub plants: usize,
}

/// Plant models baked onto terrain chunks: grass first, then the small and
/// large plants, sharing one material.
struct Plants {
    variants: Vec<vegetation::Variant>,
    grass: usize,
    material: Handle<exa_render::material::ExaMaterial>,
}

impl Plants {
    const MODELS: [&str; 3] = ["trn_grassa02.rfc", "trn_splnta02.rfc", "trn_lplnta02.rfc"];

    fn load(loader: &mut ModelLoader<'_>, palette: &Palette) -> Option<Self> {
        let mut variants = Vec::new();
        let mut grass = 0;
        for (kind, name) in Self::MODELS.iter().enumerate() {
            let model = loader.model(PackKind::Resource, name)?;
            for node in &model.nodes {
                let mut v = vegetation::Variant::default();
                for (mesh, _) in &node.parts {
                    let Some(mesh) = loader.meshes.get(mesh) else {
                        continue;
                    };
                    append_mesh(&mut v, mesh);
                }
                if v.indices.is_empty() {
                    continue;
                }
                // Grass and small plants are lit as if facing up (the
                // original's plant loader sets their normals to +Y); grass
                // is smaller.
                if kind < 2 {
                    v.normals.iter_mut().for_each(|n| *n = Vec3::Y);
                }
                if kind == 0 {
                    v.positions
                        .iter_mut()
                        .for_each(|p| *p *= vegetation::GRASS_SCALE);
                    grass += 1;
                }
                variants.push(v);
            }
        }
        let tint = palette.plants.first().map_or([0.72, 0.68, 0.76], |p| {
            std::array::from_fn(|i| p.params.get(i).copied().unwrap_or(1.0))
        });
        let base_color_texture = loader
            .texture_sync("plantset_01d", ColorSpace::Srgb)
            .map(|(t, _)| t);
        let normal_map_texture = loader
            .texture_sync("plantset_01n", ColorSpace::Linear)
            .map(|(t, _)| t);
        use exa_render::material::{ExaMaterial, ExaParams, flags};
        let material = loader.materials.add(
            ExaMaterial {
                params: ExaParams {
                    // The shader multiplies the texture by the tint as is.
                    diffuse: Vec4::new(tint[0], tint[1], tint[2], 1.0),
                    flags: flags::LIT | flags::TWO_SIDED,
                    ..default()
                },
                diffuse_map: base_color_texture,
                normal_map: normal_map_texture,
                alpha_mode: AlphaMode::Mask(0.1),
                ..default()
            }
            .with_map_flags(),
        );
        Some(Self {
            variants,
            grass,
            material,
        })
    }

    /// Grows a tile's plants as chunk meshes under `tile_entity`; returns
    /// how many grew. None grow in absent blocks (as in holes); a chunk
    /// goes to the room of its plants' block.
    #[allow(clippy::too_many_arguments)]
    fn spawn(
        &self,
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        terrain: &Terrain,
        tile: &TerrainTile,
        built: &BuiltTile,
        blocks: Option<&TileBlocks>,
        tile_entity: Entity,
    ) -> usize {
        let n = TILE_VERTICES;
        let block = |p: bevy::math::Vec2| {
            TileBlocks::of(
                (p.x.round().max(0.0) as usize).min(n - 1),
                (p.y.round().max(0.0) as usize).min(n - 1),
                n,
            )
        };
        let density = |p: bevy::math::Vec2| {
            let (hx, hy) = (
                p.x.round_ties_even() as usize,
                p.y.round_ties_even() as usize,
            );
            if tile.is_hole(hx, hy) {
                return -1.0;
            }
            if let Some(b) = blocks {
                let (bx, by) = block(p);
                if !b.has(bx, by) {
                    return -1.0;
                }
            }
            let (i, j) = vegetation::density_texel(p);
            built.density[j * n + i]
        };
        let growth = vegetation::grow(
            tile.vegetation_seed,
            self.grass,
            self.variants.len() - self.grass,
            density,
        );
        let s = terrain.spacing;
        let half = (n - 1) as f32 * 0.5 * s;
        let place = |p: bevy::math::Vec2| {
            Vec3::new(
                p.x * s - half,
                vegetation::ground(&built.heights, n, p),
                p.y * s - half,
            )
        };
        let mut grown = 0;
        for (k, chunk) in growth.iter().enumerate() {
            if chunk.is_empty() {
                continue;
            }
            grown += chunk.len();
            let baked = vegetation::bake(&self.variants, chunk, place);
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                baked
                    .positions
                    .iter()
                    .map(|p| p.to_array())
                    .collect::<Vec<_>>(),
            );
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_NORMAL,
                baked
                    .normals
                    .iter()
                    .map(|p| p.to_array())
                    .collect::<Vec<_>>(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, baked.uvs);
            mesh.insert_indices(Indices::U32(baked.indices));
            let _ = mesh.generate_tangents();
            let e = commands
                .spawn((
                    Name::new(format!("plants {}", k)),
                    PlantChunk,
                    // The original draws vegetation through its renderer
                    // hooks: it never collides and casts shadows only above
                    // shadow quality 4.
                    exa_physics::NoStaticCollider,
                    Mesh3d(loader.meshes.add(mesh)),
                    MeshMaterial3d(self.material.clone()),
                    Transform::IDENTITY,
                    Visibility::default(),
                    ChildOf(tile_entity),
                ))
                .id();
            if let Some(b) = blocks {
                // The point set is partitioned into the tile's 8 × 8 chunks:
                // chunk `k` is column `k % 8`, row `k / 8`.
                let (bx, by) = (k % vegetation::CHUNKS, k / vegetation::CHUNKS);
                if let Some(room) = b.room(bx, by) {
                    commands.entity(e).insert(InRoom(room));
                }
            }
        }
        grown
    }
}

/// A baked chunk of terrain vegetation. The original draws it through its
/// terrain's renderer hooks: no collider, and shadows only above shadow
/// quality 4.
#[derive(Component)]
pub struct PlantChunk;

/// Appends a Bevy mesh's positions, normals, UVs and indices to a variant.
fn append_mesh(v: &mut vegetation::Variant, mesh: &Mesh) {
    use bevy::mesh::VertexAttributeValues as A;
    let Some(A::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
        return;
    };
    let base = v.positions.len() as u32;
    v.positions.extend(positions.iter().map(|p| Vec3::from(*p)));
    match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(A::Float32x3(n)) => v.normals.extend(n.iter().map(|p| Vec3::from(*p))),
        _ => v
            .normals
            .extend(std::iter::repeat_n(Vec3::Y, positions.len())),
    }
    match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(A::Float32x2(uv)) => v.uvs.extend_from_slice(uv),
        _ => v
            .uvs
            .extend(std::iter::repeat_n([0.0, 0.0], positions.len())),
    }
    if let Some(indices) = mesh.indices() {
        v.indices.extend(indices.iter().map(|i| i as u32 + base));
    }
}

/// Spawns the terrain stored beside a level (`<level>.rft`), if any, under
/// `parent`, all at once ([`TerrainBuild`] spreads the same work over
/// steps).
pub fn spawn_terrain(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    level: &str,
    parent: Entity,
) -> Option<TerrainStats> {
    let mut build = TerrainBuild::begin(loader, level, parent, None)?;
    while !build.step(commands, loader, std::time::Duration::MAX) {
        std::thread::yield_now();
    }
    Some(build.stats)
}

/// A level's terrain spawned a part at a time, so a level load can keep its
/// frames short. Each part is one tile's heightfield and weights (built on
/// the async compute pool), one palette material's textures (loaded the
/// first time a tile uses it), or spawning a built tile with its blocks and
/// plants.
pub struct TerrainBuild {
    terrain: Arc<Terrain>,
    palette: Arc<Palette>,
    tree: Arc<RuleTree>,
    noise: Arc<NoiseMaps>,
    environment: Option<Handle<Image>>,
    /// The stand-in image carrying the layers' repeating sampler.
    layer_sampler: Handle<Image>,
    /// The palette's height and mask textures, by lower-case name, for the
    /// tile worker to decode.
    texture_bytes: Arc<HashMap<String, Vec<u8>>>,
    /// Height and mask planes decoded by the worker, by texture name.
    plane_cache: Arc<Mutex<HashMap<String, Option<Arc<Plane>>>>>,
    /// A tile's geometry and masks being built on the async pool, with the
    /// tile's index: the next tile, or the one after it while the next
    /// spawns.
    building: Option<(usize, Task<BuiltTile>)>,
    plants: Option<Plants>,
    /// Each palette material's diffuse, surface and normal textures.
    textures: HashMap<usize, MaterialTextures>,
    /// Surface maps being packed, by palette material.
    surfaces: HashMap<usize, Surface>,
    /// Bytes of textures made in this step.
    uploaded: usize,
    /// The tile being built, once its heightfield is.
    built: Option<BuiltTile>,
    /// In a tile-grid level, which blocks of each tile exist.
    blocks: Option<Vec<TileBlocks>>,
    /// Each tile's ground, shared with the navigation grid and the world
    /// height query.
    grounds: Vec<Option<Arc<ground::GroundTile>>>,
    /// The original's composite pools, sized from the texture-detail
    /// setting at load. Exarust layers the materials per pixel, so no set is
    /// taken; the pool keeps the original's counts, levels and sizes.
    composite: composite::CompositePools,
    /// Whether the palette's detail texture exists. The original tries to
    /// load it once and never samples it; the shipped install has no such
    /// entry.
    detail_texture: bool,
    /// Each tile's streaming bits, as the original's worker and frame hook
    /// keep them. The loading screen inserts every tile as it finishes, so
    /// the frame hook itself has no work after a load.
    stream: stream::TerrainStream,
    /// The wireframe overlay's material, when `EXA_TERRAIN_WIRE` is set.
    wireframe: Option<Handle<exa_render::material::ExaMaterial>>,
    /// Whether the world height query has been given to the world.
    ground_inserted: bool,
    /// Each tile's spawned entity and block entities, for the live frame
    /// hook and the composite selection.
    live_tiles: Vec<Option<live::LiveTile>>,
    /// Every spawned block, in the selector's order.
    live_blocks: Vec<live::LiveBlock>,
    next: usize,
    parent: Entity,
    stats: TerrainStats,
}

/// Bytes of textures one step makes at most (at least one material's): they
/// are uploaded to the GPU the frame after, and a level's palette at once
/// stalled a frame.
const UPLOAD_BUDGET: usize = 24 << 20;

/// Terrain vegetation casts shadows only above shadow quality 4, as the
/// original's grass plug-in installs its sun-shadow hook there. The chunk
/// entities stay; the flag moves with the settings.
fn sync_plant_shadows(
    quality: Res<exa_render::quality::ShadowQuality>,
    mut commands: Commands,
    chunks: Query<(Entity, Has<bevy::light::NotShadowCaster>), With<PlantChunk>>,
    added: Query<(), Added<PlantChunk>>,
) {
    if !quality.is_changed() && added.is_empty() {
        return;
    }
    let cast = quality.0 > 4;
    for (e, no_cast) in &chunks {
        if cast && no_cast {
            commands.entity(e).remove::<bevy::light::NotShadowCaster>();
        } else if !cast && !no_cast {
            commands.entity(e).insert(bevy::light::NotShadowCaster);
        }
    }
}

/// A palette material's surface map, packed on the async compute pool:
/// decoding, packing and filtering three maps took over 100 ms.
enum Surface {
    Packing(Task<Image>),
    Ready(Box<Image>),
}

impl Surface {
    /// Starts packing `material`'s surface map from copies of its maps.
    fn start(game: &exa_assets::GameData, material: &PaletteMaterial) -> Self {
        let bytes = |slot| {
            material
                .texture(slot)
                .and_then(|name| game.texture(name))
                .map(<[u8]>::to_vec)
        };
        let maps = [
            bytes(TerrainTexture::Bump),
            bytes(TerrainTexture::Gloss),
            bytes(TerrainTexture::Mask),
        ];
        match AsyncComputeTaskPool::try_get() {
            Some(pool) => Self::Packing(pool.spawn(async move { surface_image(&maps) })),
            None => Self::Ready(Box::new(surface_image(&maps))),
        }
    }

    /// The packed map, once it is done.
    fn poll(&mut self) -> Option<Image> {
        match self {
            Self::Packing(task) => block_on(poll_once(task)),
            Self::Ready(image) => Some(std::mem::take(&mut **image)),
        }
    }
}

/// A palette material's diffuse, surface and normal textures.
type MaterialTextures = (
    Option<Handle<Image>>,
    Option<Handle<Image>>,
    Option<Handle<Image>>,
);

/// The palette a terrain loads: its own `<name>.ftb`, else, only when that
/// file is missing, `default.ftb` (the original logs "Could not find
/// <name>.ftb" under "Terrain Palette" and falls back). A palette that is
/// there but does not parse is not replaced: the original's loader fails
/// and the terrain has no palette. Returns the palette, whether the fallback
/// was used, and the messages.
fn choose_palette<'a>(
    name: &str,
    mut lookup: impl FnMut(&str) -> Option<&'a [u8]>,
) -> (Option<Palette>, bool, Vec<String>) {
    let mut warnings = Vec::new();
    let entry = format!("{name}.ftb");
    let (entry, bytes, fallback) = match lookup(entry.as_str()) {
        Some(bytes) => (entry, Some(bytes), false),
        None => {
            warnings.push(format!("Could not find {entry} (Terrain Palette)"));
            let default = "default.ftb".to_string();
            let bytes = lookup(default.as_str());
            (default, bytes, true)
        }
    };
    let palette = bytes.and_then(|bytes| match Palette::parse(bytes) {
        Ok(p) => Some(p),
        Err(e) => {
            warnings.push(format!("terrain palette {entry}: {e}"));
            None
        }
    });
    (palette, fallback, warnings)
}

impl TerrainBuild {
    /// Reads the terrain beside `level` and its palette; `None` when the
    /// level has no terrain or it does not parse. In a level with a tile
    /// grid only the blocks the grid flags are built, each in its room.
    pub fn begin(
        loader: &mut ModelLoader<'_>,
        level: &str,
        parent: Entity,
        grid: Option<(&LevelGrid, Arc<Rooms>)>,
    ) -> Option<Self> {
        let stem = level.strip_suffix(".rfc").unwrap_or(level);
        let resource = loader.game.pack(PackKind::Resource)?;
        let bytes = resource.get(&format!("{stem}.rft"))?;
        let terrain = match Terrain::parse(bytes) {
            Ok(t) => t,
            Err(e) => {
                loader.warn_once(format!("terrain {stem}.rft: {e}"));
                return None;
            }
        };
        let (palette, _used_default, warnings) =
            choose_palette(&terrain.palette, |name| resource.get(name));
        for warning in warnings {
            loader.warn_once(warning);
        }
        let palette = palette?;
        let tree = RuleTree::new(&palette);
        let blocks = grid.map(|(grid, rooms)| grid_blocks(&terrain, grid, &rooms));
        let environment = loader.environment_map();
        let layer_sampler = loader.images.add(layer_sampler_image());
        let plants = Plants::load(loader, &palette);
        let tile_count = terrain.tiles.len();
        let detail_texture = loader.game.texture("trndtl01d").is_some();
        let wireframe = std::env::var_os("EXA_TERRAIN_WIRE").map(|_| {
            loader.materials.add(exa_render::material::ExaMaterial {
                params: exa_render::material::ExaParams {
                    diffuse: Vec4::new(1.0, 0.55, 0.1, 1.0),
                    ..default()
                },
                ..default()
            })
        });
        // The tile worker decodes height and mask planes itself; copy their
        // bytes now, while the install is at hand.
        let mut texture_bytes = HashMap::new();
        for material in &palette.materials {
            for slot in [TerrainTexture::Height, TerrainTexture::Mask] {
                if let Some(name) = material.texture(slot)
                    && let Some(bytes) = loader.game.texture(name)
                {
                    texture_bytes
                        .entry(name.to_ascii_lowercase())
                        .or_insert_with(|| bytes.to_vec());
                }
            }
        }
        Some(Self {
            terrain: Arc::new(terrain),
            palette: Arc::new(palette),
            tree: Arc::new(tree),
            noise: Arc::new(NoiseMaps::default()),
            environment,
            layer_sampler,
            texture_bytes: Arc::new(texture_bytes),
            plane_cache: Arc::new(Mutex::new(HashMap::new())),
            building: None,
            plants,
            textures: HashMap::new(),
            surfaces: HashMap::new(),
            uploaded: 0,
            built: None,
            blocks,
            grounds: vec![None; tile_count],
            composite: composite::CompositePools::new(
                composite::detail_size(manager::ManagerSettings::DEFAULT.texture_detail),
                composite::CompositeFeatures::default(),
            ),
            detail_texture,
            stream: stream::TerrainStream::new(vec![stream::TileStream::raw(); tile_count]),
            wireframe,
            ground_inserted: false,
            live_tiles: vec![None; tile_count],
            live_blocks: Vec::new(),
            next: 0,
            parent,
            stats: TerrainStats::default(),
        })
    }

    /// The low and high corners of the terrain's tiles on the X/Z plane,
    /// metres (tile centres ± half a tile); `None` without tiles.
    pub fn horizontal_extent(&self) -> Option<(Vec2, Vec2)> {
        let half = self.terrain.spacing * (TILE_VERTICES - 1) as f32 * 0.5;
        self.terrain.tiles.iter().fold(None, |extent, tile| {
            let centre = Vec2::from(self.terrain.tile_centre(tile));
            let (low, high) = (centre - half, centre + half);
            Some(match extent {
                Some((l, h)) => (low.min(l), high.max(h)),
                None => (low, high),
            })
        })
    }

    /// What the terrain spawned so far.
    pub fn stats(&self) -> &TerrainStats {
        &self.stats
    }

    /// The composite set pool the original sizes at load.
    pub fn composite_pools(&self) -> &composite::CompositePools {
        &self.composite
    }

    /// Whether the palette's detail texture is installed. The original loads
    /// it once; no pass samples it.
    pub fn detail_texture_loaded(&self) -> bool {
        self.detail_texture
    }

    /// Each tile's streaming bits.
    pub fn stream(&self) -> &stream::TerrainStream {
        &self.stream
    }

    /// Does parts of the terrain until `budget` has passed (at least one);
    /// `true` once every tile is spawned.
    pub fn step(
        &mut self,
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        budget: std::time::Duration,
    ) -> bool {
        let started = std::time::Instant::now();
        self.uploaded = 0;
        // The frame hook's one tile a step, as the original runs it from the
        // terrain's activation hook.
        self.frame_hook(commands);
        while self.next < self.terrain.tiles.len() {
            // A surface map still being packed waits for a later step, and
            // so do textures past the step's upload budget.
            if !self.part(commands, loader)
                || started.elapsed() >= budget
                || self.uploaded >= UPLOAD_BUDGET
            {
                break;
            }
        }
        let done = self.next >= self.terrain.tiles.len();
        if done && !self.ground_inserted {
            let mut field = ground::TerrainGround::new(
                self.terrain.width,
                self.terrain.height,
                self.terrain.spacing,
            );
            for ground in self.grounds.iter().flatten() {
                field.insert(ground.clone());
            }
            commands.insert_resource(field);
            self.ground_inserted = true;
            // The block-until-loaded wait: every built tile is inserted
            // before the level shows.
            self.flush_insertions(commands);
            commands.insert_resource(live::TerrainStreaming::new(
                self.stream.clone(),
                self.composite.clone(),
                self.live_tiles.clone(),
                self.live_blocks.clone(),
            ));
        }
        done
    }

    /// Applies one frame-hook action to the tile's entity: an insert shows
    /// the tile's nodes, an unload hides them.
    fn apply_tile_action(&self, commands: &mut Commands, tile: usize, action: stream::TileAction) {
        let Some(Some(live)) = self.live_tiles.get(tile) else {
            return;
        };
        match action {
            stream::TileAction::Insert => {
                commands.entity(live.entity).insert(Visibility::Visible);
            }
            stream::TileAction::Unload => {
                commands.entity(live.entity).insert(Visibility::Hidden);
            }
            stream::TileAction::Upload | stream::TileAction::None => {}
        }
    }

    /// One call of the original's frame hook, handling at most one tile.
    fn frame_hook(&mut self, commands: &mut Commands) {
        let Some(step) = self.stream.frame() else {
            return;
        };
        if let Some((tile, action)) = step.handled {
            self.apply_tile_action(commands, tile, action);
        }
    }

    /// Runs the frame hook until every tile is inserted, as the original's
    /// block-until-loaded slot does before a level is shown.
    fn flush_insertions(&mut self, commands: &mut Commands) {
        let mut actions = Vec::new();
        self.stream
            .wait_until_inserted(|tile, action| actions.push((tile, action)));
        for (tile, action) in actions {
            self.apply_tile_action(commands, tile, action);
        }
    }

    /// The first tile from `index` on that is built (in a grid level, one
    /// with a block).
    fn next_buildable(&self, index: usize) -> Option<usize> {
        (index..self.terrain.tiles.len()).find(|&i| self.blocks.as_ref().is_none_or(|b| b[i].any()))
    }

    /// Starts building tile `index`'s geometry and masks on the async pool,
    /// or, without the pool, builds it here.
    fn start_build(&self, index: usize) -> Result<Task<BuiltTile>, Box<BuiltTile>> {
        let terrain = self.terrain.clone();
        let palette = self.palette.clone();
        let tree = self.tree.clone();
        let noise = self.noise.clone();
        let blocks = self.blocks.as_ref().map(|b| b[index].clone());
        let bytes = self.texture_bytes.clone();
        let cache = self.plane_cache.clone();
        let build = move || {
            let tile = &terrain.tiles[index];
            let mut planes = cache.lock().unwrap_or_else(|e| e.into_inner());
            let mut height_layers =
                |used: &[usize]| height_layers(&palette, &mut planes, &bytes, used);
            build_tile(
                &terrain,
                tile,
                &palette,
                &tree,
                &noise,
                &mut height_layers,
                blocks.as_ref(),
            )
        };
        match AsyncComputeTaskPool::try_get() {
            Some(pool) => Ok(pool.spawn(async move { build() })),
            None => Err(Box::new(build())),
        }
    }

    /// One part of the next tile: its heightfield, a material's textures,
    /// or the tile itself; `false` when it waits for the tile worker or a
    /// surface map.
    fn part(&mut self, commands: &mut Commands, loader: &mut ModelLoader<'_>) -> bool {
        if self.built.is_none() {
            // The original's terrain worker builds a tile's geometry and
            // masks off the main thread; the loading screen keeps drawing
            // while it does. A tile without a block in a grid level is not
            // built.
            if self.blocks.as_ref().is_some_and(|b| !b[self.next].any()) {
                // A tile with no block has nothing to insert: it is built
                // and inserted at once, so the block-until-loaded wait ends.
                self.stream.tiles[self.next].mark_built(false);
                self.stream.tiles[self.next].mark_inserted();
                self.next += 1;
                return true;
            }
            let started = match self.building.take() {
                Some((index, task)) if index == self.next => Ok(task),
                _ => self.start_build(self.next),
            };
            let built = match started {
                Err(built) => *built,
                Ok(mut task) => {
                    let Some(built) = block_on(poll_once(&mut task)) else {
                        self.building = Some((self.next, task));
                        return false;
                    };
                    built
                }
            };
            self.built = Some(built);
            // The worker goes straight on to the next tile while this one's
            // textures load and it spawns.
            if AsyncComputeTaskPool::try_get().is_some()
                && let Some(next) = self.next_buildable(self.next + 1)
                && let Ok(task) = self.start_build(next)
            {
                self.building = Some((next, task));
            }
            let (unresolved, truncated, brushes_exceeded, materials) = {
                let built = self.built.as_ref().expect("tile just built");
                (
                    built.unresolved,
                    built.truncated,
                    built.brushes_exceeded,
                    built.materials.clone(),
                )
            };
            self.stats.unresolved_layers += unresolved;
            self.stats.truncated_tiles += usize::from(truncated);
            self.stats.brush_limit_tiles += usize::from(brushes_exceeded > 0);
            // Its new materials' surface maps are packed meanwhile.
            for m in materials {
                if !self.textures.contains_key(&m) && !self.surfaces.contains_key(&m) {
                    let surface = Surface::start(loader.game, &self.palette.materials[m]);
                    self.surfaces.insert(m, surface);
                }
            }
            return true;
        }
        let tile = &self.terrain.tiles[self.next];
        let blocks = self.blocks.as_ref().map(|b| &b[self.next]);
        let built = self.built.as_mut().expect("tile built");
        if let Some(&m) = built
            .materials
            .iter()
            .find(|m| !self.textures.contains_key(m))
        {
            let material = &self.palette.materials[m];
            let surface = self
                .surfaces
                .entry(m)
                .or_insert_with(|| Surface::start(loader.game, material));
            let Some(surface) = surface.poll() else {
                return false;
            };
            self.surfaces.remove(&m);
            let diffuse = material
                .texture(TerrainTexture::Diffuse)
                .and_then(|name| loader.texture_sync(name, ColorSpace::Srgb))
                .map(|(img, _)| img);
            let surface = Some(loader.images.add(surface));
            let normal = material
                .texture(TerrainTexture::Normal)
                .and_then(|name| loader.texture_sync(name, ColorSpace::Linear))
                .map(|(img, _)| img);
            let size = |image: &Option<Handle<Image>>| {
                image
                    .as_ref()
                    .and_then(|h| loader.images.get(h))
                    .and_then(|i| i.data.as_ref())
                    .map_or(0, Vec::len)
            };
            self.uploaded += size(&diffuse) + size(&surface) + size(&normal);
            self.textures.insert(m, (diffuse, surface, normal));
            return true;
        }
        let mut splat = TerrainSplat {
            environment: self.environment.clone(),
            layer_sampler: Some(self.layer_sampler.clone()),
            splat: Some(loader.images.add(splat_image(&built.splat))),
            ..default()
        };
        splat.params.layers = built.materials.len() as u32;
        splat.params.wetness = self.palette.wetness;
        if self.palette.wetness > composite::WET_FLOOR_THRESHOLD {
            splat.params.flags |= terrain_flags::WET_FLOOR;
        }
        if splat.environment.is_some() {
            splat.params.flags |= terrain_flags::ENVIRONMENT;
        }
        for (slot, &m) in built.materials.iter().enumerate() {
            let material = &self.palette.materials[m];
            let (diffuse, surface, normal) = self.textures[&m].clone();
            let masked = material.texture(TerrainTexture::Mask).is_some();
            let look = Look::of(material);
            splat.params.normal[slot] = Vec4::new(
                look.normal_sharpness[0],
                look.normal_sharpness[1],
                if normal.is_some() {
                    look.normal_strength
                } else {
                    0.0
                },
                look.gloss,
            );
            splat.params.colour[slot] = look.colour;
            splat.params.blend[slot] = Vec4::new(
                look.sharpness[0],
                look.sharpness[1],
                if masked { 1.0 } else { 0.0 },
                look.tiling,
            );
            splat.params.surface[slot] = Vec4::new(
                look.gloss,
                look.parallax[0],
                look.parallax[1],
                look.parallax[2],
            );
            if let Some([d, su, nm]) = splat.layer_mut(slot) {
                (*d, *su, *nm) = (diffuse, surface, normal);
            }
        }
        let centre = self.terrain.tile_centre(tile);
        let tile_entity = commands
            .spawn((
                Name::new(format!("terrain {} {}", tile.x, tile.y)),
                PendingSplat(splat),
                Transform::from_xyz(centre[0], 0.0, centre[1]),
                // The frame hook inserts the tile's nodes.
                Visibility::Hidden,
                ChildOf(self.parent),
            ))
            .id();
        let surface = (
            crate::gi_volume::GiTerrain,
            crate::shadow_links::Unwalled,
            exa_render::lights::ShadowBias(exa_render::shadow_bias::FLOOR),
        );
        let tile_size = self.terrain.spacing * (TILE_VERTICES - 1) as f32;
        let wireframe = self.wireframe.clone();
        let mut block_entities = Vec::new();
        for block in std::mem::take(&mut built.blocks) {
            let record = TerrainBlock {
                bx: block.bx as u32,
                by: block.by as u32,
                level: TerrainBlock::LEVEL,
                room: block.room,
            };
            let bounds = record.bounds(tile_size, Vec3::ZERO);
            let overlay = wireframe.as_ref().and_then(|material| {
                wireframe::overlay_mesh(&block.mesh)
                    .map(|mesh| (loader.meshes.add(mesh), material.clone()))
            });
            let part = commands
                .spawn((
                    Name::new(format!(
                        "terrain {} {} block {} {}",
                        tile.x, tile.y, block.bx, block.by
                    )),
                    Mesh3d(loader.meshes.add(block.mesh)),
                    TerrainPart,
                    record,
                    bounds,
                    surface,
                    Transform::IDENTITY,
                    Visibility::default(),
                    ChildOf(tile_entity),
                ))
                .id();
            if let Some(room) = block.room {
                commands.entity(part).insert(InRoom(room));
            }
            block_entities.push(part);
            let centre_in_tile =
                TerrainBlock::centre_in_tile(tile_size, record.bx, record.by, record.level);
            self.live_blocks.push(live::LiveBlock {
                entity: part,
                tile: self.next,
                centre: Vec2::new(centre[0] + centre_in_tile.x, centre[1] + centre_in_tile.z),
                half: TerrainBlock::half_extents(tile_size, record.level),
            });
            if let Some((mesh, material)) = overlay {
                commands.spawn((
                    wireframe::WireframeOverlay,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    exa_physics::NoStaticCollider,
                    bevy::light::NotShadowCaster,
                    Transform::IDENTITY,
                    Visibility::default(),
                    ChildOf(part),
                ));
            }
        }
        self.live_tiles[self.next] = Some(live::LiveTile {
            entity: tile_entity,
            blocks: block_entities,
        });
        // The navigation grid reads the ground of the blocks built here,
        // and the world height query keeps the same ground.
        let mut ground = ground::GroundTile::new(&self.terrain, tile, built.heights.clone());
        if let Some(b) = blocks {
            ground = ground.with_blocks(b.present);
        }
        let ground = Arc::new(ground);
        commands
            .entity(tile_entity)
            .insert(crate::nav::view::Ground(ground.clone()));
        let slot = self.next;
        if let Some(plants) = &self.plants {
            self.stats.plants += plants.spawn(
                commands,
                loader,
                &self.terrain,
                tile,
                built,
                blocks,
                tile_entity,
            );
        }
        self.grounds[slot] = Some(ground);
        // The tile counts as built (ready for the frame hook's insert) only
        // once its entity exists: the worker's geometry can finish steps
        // before its textures load and it spawns, and an insert handled in
        // between would find no entity to show and leave the tile hidden.
        self.stream.tiles[slot].mark_built(false);
        self.stats.tiles += 1;
        self.built = None;
        self.next += 1;
        true
    }
}

/// The height layers of the palette materials `used`, their height and mask
/// planes decoded once by name into the worker's shared cache.
fn height_layers(
    palette: &Palette,
    planes: &mut HashMap<String, Option<Arc<Plane>>>,
    bytes: &HashMap<String, Vec<u8>>,
    used: &[usize],
) -> Vec<HeightLayer> {
    let mut plane = |name: Option<&str>| -> Option<Arc<Plane>> {
        let name = name?.to_ascii_lowercase();
        if let Some(plane) = planes.get(&name) {
            return plane.clone();
        }
        let plane = bytes.get(&name).and_then(|bytes| {
            let rfi = Rfi::parse(bytes).ok()?;
            let rgba = rfi.decode_rgba8().ok()?;
            let (w, h) = (rfi.header.width as usize, rfi.header.height as usize);
            Plane::from_rgba(w, h, &rgba).map(Arc::new)
        });
        planes.insert(name, plane.clone());
        plane
    };
    used.iter()
        .map(|&m| {
            let material = &palette.materials[m];
            let p = |i: usize, d: f32| material.params.get(i).copied().unwrap_or(d);
            let look = Look::of(material);
            HeightLayer {
                base: p(16, 0.5),
                offset: p(17, 0.5),
                scale: p(18, 0.5),
                height_map: plane(material.texture(TerrainTexture::Height)),
                mask: plane(material.texture(TerrainTexture::Mask)),
                sharpness: look.sharpness,
                tiling: look.tiling,
                vegetation: p(13, 0.0),
            }
        })
        .collect()
}

/// Decodes a palette texture's base level to RGBA8 with its size.
fn decode(bytes: Option<&[u8]>) -> Option<(usize, usize, Vec<u8>)> {
    let rfi = Rfi::parse(bytes?).ok()?;
    let rgba = rfi.decode_rgba8().ok()?;
    Some((rfi.header.width as usize, rfi.header.height as usize, rgba))
}

/// A palette material's parallax height (its bump slot, red), gloss (green)
/// and blend mask (blue) in one linear texture at the largest of their
/// sizes, with a box-filtered mip chain. Missing maps read as height ½,
/// gloss 0 and mask 1. `maps` are the bump, gloss and mask maps' files.
fn surface_image(maps: &[Option<Vec<u8>>; 3]) -> Image {
    let [bump, gloss, mask] = maps.each_ref().map(|m| decode(m.as_deref()));
    let (w, h) = [&bump, &gloss, &mask]
        .iter()
        .filter_map(|m| m.as_ref().map(|(w, h, _)| (*w, *h)))
        .max()
        .unwrap_or((1, 1));
    // The red channel of `map` at (x, y) of the w × h output, or `missing`.
    let channel = |map: &Option<(usize, usize, Vec<u8>)>, x: usize, y: usize, missing: u8| {
        map.as_ref().map_or(missing, |(mw, mh, px)| {
            let (sx, sy) = (x * mw / w, y * mh / h);
            px[(sx + sy * mw) * 4]
        })
    };
    let mut level = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = (x + y * w) * 4;
            level[i] = channel(&bump, x, y, 128);
            level[i + 1] = channel(&gloss, x, y, 0);
            level[i + 2] = channel(&mask, x, y, 255);
            level[i + 3] = 255;
        }
    }
    let (mut lw, mut lh) = (w, h);
    let mut data = level.clone();
    let mut mips = 1;
    while lw > 1 || lh > 1 {
        let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut sum = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let (sx, sy) = ((x * 2 + dx).min(lw - 1), (y * 2 + dy).min(lh - 1));
                        sum += level[(sx + sy * lw) * 4 + c] as u32;
                    }
                    next[(x + y * nw) * 4 + c] = (sum / 4) as u8;
                }
            }
        }
        data.extend_from_slice(&next);
        level = next;
        (lw, lh) = (nw, nh);
        mips += 1;
    }
    let mut image = Image::new_uninit(
        bevy::render::render_resource::Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = mips;
    image.data = Some(data);
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        anisotropy_clamp: 16,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    image
}

/// Shading values of a palette material, from its parameter block.
struct Look {
    colour: Vec4,
    sharpness: [f32; 2],
    normal_sharpness: [f32; 2],
    normal_strength: f32,
    gloss: f32,
    /// Parallax height base, reference and scale (f20, f21, f22).
    parallax: [f32; 3],
    tiling: f32,
    /// The cover strength (f9): how strongly a masked layer hides what is
    /// below, and the colour pass's alpha.
    cover: f32,
    /// The height pass's base, reference and scale (f16, f17, f18).
    height: [f32; 3],
    /// The mask pass's colour (f12, f13, f14).
    mask_colour: [f32; 3],
    /// The texture repeats per tile before [`UV_PER_METRE`] (f0, or 1 when
    /// not positive).
    tile: f32,
}

impl Look {
    fn of(material: &PaletteMaterial) -> Self {
        let p = |i: usize, default: f32| material.params.get(i).copied().unwrap_or(default);
        let tint = material.tint();
        let tint = Color::srgb(tint[0], tint[1], tint[2]).to_linear();
        let cover = p(9, 1.0);
        let gain = TINT_GAIN * cover.min(1.0);
        let tile = match p(0, 1.0) {
            t if t > 0.0 => t,
            _ => 1.0,
        };
        Self {
            colour: Vec4::new(tint.red * gain, tint.green * gain, tint.blue * gain, cover),
            sharpness: [p(2, 1.0), p(3, 0.0)],
            normal_sharpness: [p(4, 1.0), p(5, 0.0)],
            normal_strength: p(11, 1.0),
            gloss: p(10, 0.25) * gain,
            parallax: [p(20, 0.5), p(21, 0.5), p(22, 1.0)],
            tiling: tile * UV_PER_METRE,
            cover,
            height: [p(16, 0.5), p(17, 0.5), p(18, 0.5)],
            mask_colour: [p(12, 0.0), p(13, 0.0), p(14, 0.0)],
            tile,
        }
    }
}

/// One 6 m block of a tile, built as its own mesh as the original builds a
/// terrain mesh per block.
struct BuiltBlock {
    bx: usize,
    by: usize,
    room: Option<u16>,
    mesh: Mesh,
}

struct BuiltTile {
    /// The tile's 6 m block meshes, each with its room; taken when the tile
    /// spawns.
    blocks: Vec<BuiltBlock>,
    /// Displaced vertex heights (metres) and vegetation density per vertex.
    heights: Vec<f32>,
    density: Vec<f32>,
    /// Palette material indices, one per layer, ascending.
    materials: Vec<usize>,
    /// The layers' over opacities per vertex, as the splat texture holds
    /// them ([`splat_image`]).
    splat: Vec<u8>,
    unresolved: usize,
    truncated: bool,
    /// Painted brushes past the original's eleven-brush limit.
    brushes_exceeded: usize,
}

fn build_tile(
    terrain: &Terrain,
    tile: &TerrainTile,
    palette: &Palette,
    tree: &RuleTree,
    noise: &NoiseMaps,
    height_layers: &mut dyn FnMut(&[usize]) -> Vec<HeightLayer>,
    blocks: Option<&TileBlocks>,
) -> BuiltTile {
    let n = TILE_VERTICES;
    let s = terrain.spacing;
    let half = (n - 1) as f32 * 0.5;

    let mut positions = Vec::with_capacity(n * n);
    let mut normals = Vec::with_capacity(n * n);
    let mut uvs = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            positions.push([
                (i as f32 - half) * s,
                tile.height(i, j),
                (j as f32 - half) * s,
            ]);
            normals.push(sobel_normal(tile, i as i32, j as i32, s).to_array());
            uvs.push([i as f32 / (n - 1) as f32, j as f32 / (n - 1) as f32]);
        }
    }

    // Material weights per vertex from the painted layers.
    let mut layers: Vec<(Option<usize>, &[u8])> = tile
        .layers
        .iter()
        .map(|l| (RuleTree::node(palette, &l.name), l.weights.as_slice()))
        .collect();
    let unresolved = layers.iter().filter(|l| l.0.is_none()).count();
    // The original keeps at most eleven painted brushes; at the next one
    // it reports "Maximum brushes exceeded." and reads no further layers.
    let mut brushes = 0;
    let mut brushes_exceeded = 0;
    let mut keep = layers.len();
    for (at, layer) in layers.iter().enumerate() {
        if layer.0.is_none() {
            continue;
        }
        brushes += 1;
        if brushes > blend::MAX_BRUSHES {
            brushes_exceeded += 1;
            keep = keep.min(at);
        }
    }
    layers.truncate(keep);
    let m = tree.material_count();
    let mut weights = vec![0.0f32; n * n * m];
    let mut totals = vec![0.0f32; m];
    let mut painted = Vec::with_capacity(layers.len());
    for j in 0..n {
        for i in 0..n {
            let v = j * n + i;
            painted.clear();
            painted.extend(
                layers
                    .iter()
                    .filter_map(|(node, w)| node.map(|node| (node, w[v] as f32 / 255.0))),
            );
            let inputs = noise.inputs(positions[v][1], normals[v][1], tile.x, tile.y, i, j);
            let out = &mut weights[v * m..(v + 1) * m];
            tree.weigh(&painted, &inputs, out);
            for (t, w) in totals.iter_mut().zip(out.iter()) {
                *t += w;
            }
        }
    }
    let threshold = MIN_TILE_SHARE * (n * n) as f32;
    let mut used: Vec<usize> = (0..m).filter(|&i| totals[i] > threshold).collect();
    let truncated = used.len() > SPLAT_LAYERS;
    used.sort_by(|a, b| totals[*b].total_cmp(&totals[*a]));
    used.truncate(SPLAT_LAYERS);
    used.sort_unstable();

    // Opacities for the shader, and the height pass moving the vertices.
    let centre = terrain.tile_centre(tile);
    let lift = height_layers(&used);
    let mut density = vec![0.0f32; n * n];
    let mut splat = vec![0u8; n * n * SPLAT_LAYERS];
    for v in 0..n * n {
        let w = &weights[v * m..(v + 1) * m];
        let kept: Vec<f32> = used.iter().map(|&i| w[i]).collect();
        let alphas = over_alphas(&kept);
        density[v] = vegetation_density(
            &lift,
            &alphas,
            positions[v][0] + centre[0],
            positions[v][2] + centre[1],
        );
        let p = &mut positions[v];
        p[1] += displacement(
            &lift,
            &alphas,
            p[0] + centre[0],
            p[2] + centre[1],
            normals[v][1],
        );
        for (k, a) in alphas.iter().enumerate() {
            splat[splat_index(k, v, n)] = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    let normals = grid_normals(&positions, n);
    let heights: Vec<f32> = positions.iter().map(|p| p[1]).collect();

    // Each 6 m block draws the original's shared block patch (serpentine
    // rows, quads split along their low-to-high diagonal) from its first
    // vertex; holes' quads are left out, and absent blocks are not built.
    // At the tile's edge the skirt below joins its block's mesh.
    let side = TileBlocks::SIDE;
    let block_at = |i: usize, j: usize| TileBlocks::of(i, j, n);
    let present = |bx: usize, by: usize| blocks.is_none_or(|b| b.has(bx, by));
    let shared = patch::GridPatch::block().flat_indices(1, n as u16);
    let cells = patch::BLOCK_CELLS as usize;
    let mut groups: Vec<Option<Vec<u32>>> = vec![None; side * side];
    for by in 0..side {
        for bx in 0..side {
            if !present(bx, by) {
                continue;
            }
            let first = (by * n + bx) * cells;
            let group = groups[by * side + bx].get_or_insert_with(Vec::new);
            for quad in shared.as_chunks::<6>().0 {
                // A quad's smallest index is its low corner.
                let low = first + *quad.iter().min().unwrap_or(&0) as usize;
                if tile.is_hole(low % n, low / n) {
                    continue;
                }
                group.extend(quad.iter().map(|&v| (first + v as usize) as u32));
            }
        }
    }
    // A skirt hanging from the tile's edge hides cracks where neighbouring
    // tiles lift a shared edge differently.
    let mut normals = normals;
    let edge: Vec<usize> = (0..n - 1)
        .chain((0..n - 1).map(|j| j * n + n - 1))
        .chain((1..n).rev().map(|i| (n - 1) * n + i))
        .chain((1..n).rev().map(|j| j * n))
        .collect();
    let first_skirt = positions.len() as u32;
    for &v in &edge {
        let mut p = positions[v];
        p[1] -= SKIRT_DEPTH;
        positions.push(p);
        normals.push(normals[v]);
        uvs.push(uvs[v]);
    }
    for k in 0..edge.len() {
        let (a, b) = (edge[k] as u32, edge[(k + 1) % edge.len()] as u32);
        // The segment's block: the one holding its lower-left vertex.
        let (i, j) = (
            (a as usize % n).min(b as usize % n),
            (a as usize / n).min(b as usize / n),
        );
        let (bx, by) = block_at(i, j);
        if !present(bx, by) {
            continue;
        }
        let (c, d) = (
            first_skirt + k as u32,
            first_skirt + ((k + 1) % edge.len()) as u32,
        );
        groups[by * side + bx]
            .get_or_insert_with(Vec::new)
            .extend_from_slice(&[a, b, c, b, d, c, a, c, b, b, c, d]);
    }

    let vertices = TileVertices {
        positions,
        normals,
        uvs,
    };
    let mut built = Vec::new();
    for by in 0..side {
        for bx in 0..side {
            let Some(indices) = groups[by * side + bx].take().filter(|i| !i.is_empty()) else {
                continue;
            };
            built.push(BuiltBlock {
                bx,
                by,
                room: blocks.and_then(|b| b.room(bx, by)),
                mesh: vertices.part(&indices),
            });
        }
    }
    BuiltTile {
        blocks: built,
        heights,
        density,
        materials: used,
        splat,
        unresolved,
        truncated,
        brushes_exceeded,
    }
}

/// Where layer `k`'s opacity at vertex `v` (of a tile `n` vertices a side)
/// lies in the splat texture's bytes: channel k % 4 of layer k / 4.
fn splat_index(k: usize, v: usize, n: usize) -> usize {
    ((k / SPLAT_CHANNELS) * n * n + v) * SPLAT_CHANNELS + k % SPLAT_CHANNELS
}

/// A tile's splat texture: its layers' over opacities, one texel per
/// vertex, read bilinearly between them.
fn splat_image(data: &[u8]) -> Image {
    let n = TILE_VERTICES as u32;
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: n,
            height: n,
            depth_or_array_layers: (SPLAT_LAYERS / SPLAT_CHANNELS) as u32,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data.to_vec(),
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(bevy::render::render_resource::TextureViewDescriptor {
        dimension: Some(bevy::render::render_resource::TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
        address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    image
}

/// A 1 × 1 image whose sampler every layer texture is read through:
/// repeating, linear (the shader reads the base level).
fn layer_sampler_image() -> Image {
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        vec![255; 4],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    image
}

/// A tile's vertex arrays, its skirt included.
struct TileVertices {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
}

impl TileVertices {
    /// A mesh of all the vertices and these triangles.
    fn mesh(self, indices: Vec<u32>) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }

    /// A mesh of these triangles and only the vertices they use.
    fn part(&self, indices: &[u32]) -> Mesh {
        let mut remap = vec![u32::MAX; self.positions.len()];
        let mut kept = TileVertices {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
        };
        let indices = indices
            .iter()
            .map(|&v| {
                let v = v as usize;
                if remap[v] == u32::MAX {
                    remap[v] = kept.positions.len() as u32;
                    kept.positions.push(self.positions[v]);
                    kept.normals.push(self.normals[v]);
                    kept.uvs.push(self.uvs[v]);
                }
                remap[v]
            })
            .collect();
        kept.mesh(indices)
    }
}

/// Which of a terrain tile's 8 × 8 blocks of 6 m exist in a tile-grid
/// level, and each block's room.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TileBlocks {
    /// Bit `bx` of row `by` (along X and Z).
    pub present: [u8; 8],
    pub rooms: [[Option<u16>; 8]; 8],
}

impl TileBlocks {
    /// Blocks along a tile's side.
    pub const SIDE: usize = 8;

    pub fn has(&self, bx: usize, by: usize) -> bool {
        self.present[by] & (1 << bx) != 0
    }

    pub fn room(&self, bx: usize, by: usize) -> Option<u16> {
        self.rooms[by][bx]
    }

    /// Whether any block of the tile exists.
    pub fn any(&self) -> bool {
        self.present.iter().any(|&row| row != 0)
    }

    /// The block holding vertex or quad (`i`, `j`) of a tile `n` vertices
    /// a side.
    fn of(i: usize, j: usize, n: usize) -> (usize, usize) {
        let per = ((n - 1) / Self::SIDE).max(1);
        ((i / per).min(Self::SIDE - 1), (j / per).min(Self::SIDE - 1))
    }
}

/// The blocks of each of `terrain`'s tiles in a level with tile grid
/// `grid`, as the original builds a terrain in a grid level: a block exists
/// only where its grid cell carries the terrain flag. Block (bx, by) of tile
/// (tx, ty) is cell (x0 + 8·tx + bx, y0 + 8·ty + by), with x0 = (grid width −
/// 8 × tiles across) / 2 and y0 likewise, each halved towards zero. A block
/// belongs to the room of its centre.
pub fn grid_blocks(terrain: &Terrain, grid: &LevelGrid, rooms: &Rooms) -> Vec<TileBlocks> {
    let side = TileBlocks::SIDE as i64;
    let x0 = (grid.width as i64 - side * terrain.width as i64) / 2;
    let y0 = (grid.height as i64 - side * terrain.height as i64) / 2;
    let tile_m = terrain.spacing * (TILE_VERTICES - 1) as f32;
    let block_m = tile_m / TileBlocks::SIDE as f32;
    terrain
        .tiles
        .iter()
        .map(|tile| {
            let mut blocks = TileBlocks::default();
            let centre = terrain.tile_centre(tile);
            for by in 0..TileBlocks::SIDE {
                for bx in 0..TileBlocks::SIDE {
                    let x = x0 + side * tile.x as i64 + bx as i64;
                    let y = y0 + side * tile.y as i64 + by as i64;
                    let flagged = x >= 0
                        && y >= 0
                        && grid
                            .cell(x as u32, y as u32)
                            .is_some_and(|c| grid.edges[c] & CELL_TERRAIN != 0);
                    if !flagged {
                        continue;
                    }
                    blocks.present[by] |= 1 << bx;
                    let at = Vec2::new(
                        centre[0] - tile_m / 2.0 + (bx as f32 + 0.5) * block_m,
                        centre[1] - tile_m / 2.0 + (by as f32 + 0.5) * block_m,
                    );
                    blocks.rooms[by][bx] = rooms.room_at(at);
                }
            }
            blocks
        })
        .collect()
}

/// Normals of a displaced `n` × `n` vertex grid from central differences
/// (one-sided at the edges).
fn grid_normals(positions: &[[f32; 3]], n: usize) -> Vec<[f32; 3]> {
    let at = |i: usize, j: usize| Vec3::from_array(positions[j * n + i]);
    (0..n * n)
        .map(|v| {
            let (i, j) = (v % n, v / n);
            let dx = at((i + 1).min(n - 1), j) - at(i.saturating_sub(1), j);
            let dz = at(i, (j + 1).min(n - 1)) - at(i, j.saturating_sub(1));
            dz.cross(dx).normalize_or(Vec3::Y).to_array()
        })
        .collect()
}

/// Surface normal from a 3 × 3 Sobel filter over the heights, reaching into
/// the apron at tile edges.
fn sobel_normal(tile: &TerrainTile, i: i32, j: i32, spacing: f32) -> Vec3 {
    let h = |di: i32, dj: i32| tile.height_apron(i + di, j + dj);
    let gx = (h(1, -1) + 2.0 * h(1, 0) + h(1, 1)) - (h(-1, -1) + 2.0 * h(-1, 0) + h(-1, 1));
    let gz = (h(-1, 1) + 2.0 * h(0, 1) + h(1, 1)) - (h(-1, -1) + 2.0 * h(0, -1) + h(1, -1));
    let scale = 1.0 / (8.0 * spacing);
    Vec3::new(-gx * scale, 1.0, -gz * scale).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terrain_block_reports_its_flat_slab() {
        // The original's 48 m tile, eight blocks a side.
        let size = 48.0;
        let (step, half) = TerrainBlock::step(size, TerrainBlock::LEVEL);
        assert_eq!((step, half), (6.0, 3.0));
        let first = TerrainBlock::centre_in_tile(size, 0, 0, TerrainBlock::LEVEL);
        assert_eq!(first, Vec3::new(-21.0, 0.0, -21.0));
        let last = TerrainBlock::centre_in_tile(size, 7, 7, TerrainBlock::LEVEL);
        assert_eq!(last, Vec3::new(21.0, 0.0, 21.0));
        let middle = TerrainBlock::centre_in_tile(size, 3, 4, TerrainBlock::LEVEL);
        assert_eq!(middle, Vec3::new(-3.0, 0.0, 3.0));
        let half_extents = TerrainBlock::half_extents(size, TerrainBlock::LEVEL);
        assert_eq!(half_extents, Vec3::new(3.0, 0.1, 3.0));
        assert!((TerrainBlock::radius(size, TerrainBlock::LEVEL) - 18.01f32.sqrt()).abs() < 1e-6);
        let block = TerrainBlock {
            bx: 3,
            by: 4,
            level: TerrainBlock::LEVEL,
            room: Some(2),
        };
        let bounds = block.bounds(size, Vec3::new(100.0, -1.0, 50.0));
        assert_eq!(bounds.centre, Vec3::new(97.0, -1.0, 53.0));
        assert_eq!(bounds.half_extents, half_extents);
        assert_eq!(block.shadow_bias(), exa_render::shadow_bias::FLOOR);
        assert_eq!(block.room, Some(2));
        // Every block centre is on the 6 m lattice from -21 to 21.
        for by in 0..8 {
            for bx in 0..8 {
                let c = TerrainBlock::centre_in_tile(size, bx, by, TerrainBlock::LEVEL);
                assert_eq!((c.x - 6.0 * bx as f32 + 21.0).abs(), 0.0);
                assert_eq!((c.z - 6.0 * by as f32 + 21.0).abs(), 0.0);
            }
        }
    }

    #[test]
    fn look_follows_the_parameter_block() {
        let mut params = vec![0.0f32; 29];
        params[0] = 2.0;
        params[2] = 3.0;
        params[3] = 1.0;
        params[6..10].copy_from_slice(&[1.0, 1.0, 1.0, 0.5]);
        let m = PaletteMaterial {
            name: "Mat".into(),
            flags: 0,
            textures: Default::default(),
            params,
        };
        let look = Look::of(&m);
        assert!((look.colour.x - 0.75).abs() < 1e-5);
        assert_eq!(look.colour.w, 0.5);
        assert_eq!(look.sharpness, [3.0, 1.0]);
        assert_eq!(look.normal_strength, 0.0);
        assert!((look.tiling - 2.0 / 12.0).abs() < 1e-6);
    }

    /// A flat terrain of `w` × `h` tiles, all present.
    fn flat_terrain(w: u32, h: u32) -> Terrain {
        let u32s = |v: &[u32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        let mut b = u32s(&[exa_formats::terrain::TERRAIN_MAGIC, 0]);
        b.extend([0u8; 16]);
        b.extend(20f32.to_le_bytes());
        b.extend(u32s(&[w, h, w * h]));
        for y in 0..h {
            for x in 0..w {
                let mut t = u32s(&[0, x, y, 0]);
                t.extend(vec![0u8; 244 * 244 * 4]);
                b.extend(u32s(&[exa_formats::terrain::TILE, t.len() as u32]));
                b.extend(t);
            }
        }
        Terrain::parse(&b).unwrap()
    }

    fn grid(w: u32, h: u32) -> LevelGrid {
        let n = (w * h) as usize;
        LevelGrid {
            wall_sets: Vec::new(),
            floor_sets: Vec::new(),
            width: w,
            height: h,
            edges: vec![0; n],
            plane_b: vec![0; n],
            plane_c: vec![0xF; n],
        }
    }

    #[test]
    fn grid_levels_build_only_flagged_blocks() {
        // A grid two cells wider and taller than one tile: the tile's
        // block (0, 0) is cell (1, 1), block (7, 7) cell (8, 8).
        let terrain = flat_terrain(1, 1);
        let mut g = grid(10, 10);
        let open_walled = crate::rooms::CELL_OPEN | 0x7777;
        g.edges[11] = open_walled | CELL_TERRAIN;
        g.edges[88] = CELL_TERRAIN;
        g.edges[0] = CELL_TERRAIN;
        let rooms = Rooms::flood(&g);
        let blocks = grid_blocks(&terrain, &g, &rooms);
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert!(b.has(0, 0) && b.has(7, 7));
        assert_eq!(b.present.iter().map(|r| r.count_ones()).sum::<u32>(), 2);
        assert_eq!((b.room(0, 0), b.room(7, 7)), (Some(0), None));
        // Narrower than the tile: the offset halves towards zero, so cell
        // (0, 0) is block (0, 0), not block (1, 1).
        let mut g = grid(7, 7);
        g.edges[0] = CELL_TERRAIN;
        let blocks = grid_blocks(&terrain, &g, &Rooms::flood(&g));
        assert!(blocks[0].has(0, 0));
        assert!(!blocks[0].has(1, 1));
        assert!(!grid_blocks(&terrain, &grid(8, 8), &Rooms::default())[0].any());
    }

    /// A palette's bytes: the 144-byte header (magic, name) and empty
    /// material, rule and plant lists.
    fn empty_palette(name: &str) -> Vec<u8> {
        let mut b = vec![0u8; 144];
        b[..4].copy_from_slice(&exa_formats::terrain::PALETTE_MAGIC_V3.to_le_bytes());
        b[8..8 + name.len()].copy_from_slice(name.as_bytes());
        b.extend(0u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b
    }

    #[test]
    fn a_missing_palette_falls_back_to_default() {
        let bytes = empty_palette("Default");
        let (p, used, warnings) = choose_palette("missing", |name| {
            (name == "default.ftb").then_some(bytes.as_slice())
        });
        assert!(p.is_some() && used);
        assert!(
            warnings[0].contains("Could not find missing.ftb"),
            "{warnings:?}"
        );
        let (p, used, _) = choose_palette("missing", |name| {
            (name == "missing.ftb").then_some(bytes.as_slice())
        });
        assert!(p.is_some() && !used);
        // Without a default either, only the missing file is reported.
        let (p, used, warnings) = choose_palette("missing", |_| None);
        assert!(p.is_none() && used);
        assert_eq!(warnings.len(), 1);
        // A palette that is there but does not parse is not replaced: the
        // original falls back only when the file is missing.
        let (p, used, warnings) = choose_palette("bad", |name| match name {
            "bad.ftb" => Some(&[0u8; 4][..]),
            _ => Some(bytes.as_slice()),
        });
        assert!(p.is_none() && !used);
        assert!(
            warnings[0].contains("terrain palette bad.ftb"),
            "{warnings:?}"
        );
    }

    #[test]
    fn blocks_cover_thirty_quads_and_parts_keep_their_vertices() {
        let n = TILE_VERTICES;
        assert_eq!(TileBlocks::of(0, 29, n), (0, 0));
        assert_eq!(TileBlocks::of(30, 239, n), (1, 7));
        assert_eq!(TileBlocks::of(240, 240, n), (7, 7));
        let v = TileVertices {
            positions: (0..4).map(|i| [i as f32, 0.0, 0.0]).collect(),
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: vec![[0.0; 2]; 4],
        };
        let part = v.part(&[3, 1, 2, 2, 1, 3]);
        let p = part
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap();
        assert_eq!(p, [[3.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]]);
        let i: Vec<usize> = part.indices().unwrap().iter().collect();
        assert_eq!(i, [0, 1, 2, 2, 1, 0]);
    }

    /// A tile becomes one mesh per 6 m block, each covering its own 30 × 30
    /// cells (plus its share of the edge skirt), and each in its block's
    /// room.
    #[test]
    fn a_tile_is_built_as_six_metre_blocks() {
        use exa_formats::terrain::Palette;
        let terrain = flat_terrain(1, 1);
        let tile = &terrain.tiles[0];
        let palette = Palette {
            name: "p".into(),
            environment: 0.0,
            wetness: 0.0,
            materials: Vec::new(),
            rules: Vec::new(),
            plants: Vec::new(),
        };
        let tree = RuleTree::new(&palette);
        let noise = NoiseMaps::default();
        let built = build_tile(
            &terrain,
            tile,
            &palette,
            &tree,
            &noise,
            &mut |_| Vec::new(),
            None,
        );
        assert_eq!(built.blocks.len(), 64);
        for block in &built.blocks {
            let indices = block.mesh.indices().unwrap().len();
            // 30 × 30 quads, and the skirt of the blocks on the edge.
            assert!(indices >= 30 * 30 * 6, "{},{}", block.bx, block.by);
            let positions = block
                .mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3())
                .unwrap();
            let (x0, x1) = (-24.0 + 6.0 * block.bx as f32, -18.0 + 6.0 * block.bx as f32);
            let (z0, z1) = (-24.0 + 6.0 * block.by as f32, -18.0 + 6.0 * block.by as f32);
            for p in positions {
                assert!(p[0] >= x0 - 1e-3 && p[0] <= x1 + 1e-3, "x {p:?}");
                assert!(p[2] >= z0 - 1e-3 && p[2] <= z1 + 1e-3, "z {p:?}");
            }
            assert_eq!(block.room, None);
        }
        // The four corner blocks carry the whole perimeter skirt.
        let skirt: usize = built
            .blocks
            .iter()
            .map(|b| b.mesh.indices().unwrap().len() - 30 * 30 * 6)
            .sum();
        assert_eq!(skirt, 4 * (TILE_VERTICES - 1) * 12);

        // In a grid level only the present block exists, in its room.
        let mut blocks = TileBlocks::default();
        blocks.present[2] = 1 << 3;
        blocks.rooms[2][3] = Some(7);
        let built = build_tile(
            &terrain,
            tile,
            &palette,
            &tree,
            &noise,
            &mut |_| Vec::new(),
            Some(&blocks),
        );
        assert_eq!(built.blocks.len(), 1);
        assert_eq!((built.blocks[0].bx, built.blocks[0].by), (3, 2));
        assert_eq!(built.blocks[0].room, Some(7));
    }
    #[test]
    fn splat_texels_hold_each_layer_in_its_channel() {
        let n = TILE_VERTICES;
        assert_eq!(splat_index(0, 0, n), 0);
        assert_eq!(splat_index(3, 1, n), 4 + 3);
        // Layer 5: channel 1 of the second array layer.
        assert_eq!(splat_index(5, 7, n), (n * n + 7) * 4 + 1);
        assert_eq!(splat_index(15, n * n - 1, n), 4 * n * n * 4 - 1);
        let image = splat_image(&vec![0; n * n * SPLAT_LAYERS]);
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 4);
    }

    /// Every shipped tile's materials fit the layers, so none is dropped:
    /// exanima01 and exanima67 hold the tiles with the most (skipped
    /// without the game).
    #[test]
    fn shipped_tiles_keep_every_material() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Resource).unwrap();
        for level in ["exanima01", "exanima67"] {
            let terrain = Terrain::parse(pack.get(&format!("{level}.rft")).unwrap()).unwrap();
            let palette =
                Palette::parse(pack.get(&format!("{}.ftb", terrain.palette)).unwrap()).unwrap();
            let tree = RuleTree::new(&palette);
            let noise = NoiseMaps::default();
            let mut most = 0;
            for tile in &terrain.tiles {
                let built = build_tile(
                    &terrain,
                    tile,
                    &palette,
                    &tree,
                    &noise,
                    &mut |_| Vec::new(),
                    None,
                );
                assert!(!built.truncated, "{level} tile {} {}", tile.x, tile.y);
                most = most.max(built.materials.len());
            }
            // exanima01's dirt rooms stack eight; one exanima67 tile twelve.
            assert!(most > 4, "{level}: {most}");
        }
    }

    /// Every shipped material's tiling factor is a whole number of 4/48 m
    /// repeats, so the original's tile-corner UV phase and Exarust's world
    /// space sample the same texels (the corner is 24 m from the origin, a
    /// whole 2·f0 repeats away). A fractional factor would need the tile
    /// corner folded in (skipped without the game).
    #[test]
    fn shipped_material_tiling_is_whole() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Resource).unwrap();
        let mut materials = 0;
        for entry in pack.entries() {
            if !entry.name.to_ascii_lowercase().ends_with(".ftb") {
                continue;
            }
            let palette = Palette::parse(pack.bytes(entry)).unwrap();
            for material in &palette.materials {
                materials += 1;
                let f0 = material.params.first().copied().unwrap_or(0.0);
                assert_eq!(f0.fract(), 0.0, "{} {}", entry.name, material.name);
            }
        }
        assert!(materials > 100, "{materials}");
    }

    /// The detail texture the original loads once parses as an image (its
    /// handle is never read again by the terrain draw; skipped without the
    /// game).
    #[test]
    fn shipped_detail_texture_parses() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let Some(bytes) = game.texture("trndtl01d") else {
            return;
        };
        let rfi = Rfi::parse(bytes).unwrap();
        assert!(rfi.header.width > 0 && rfi.header.height > 0);
        assert!(rfi.decode_rgba8().is_ok());
    }

    /// The palette's two header floats: every shipped palette is dry but
    /// the sewer, whose wet-floor source is 0.5 (skipped without the game).
    #[test]
    fn shipped_palette_wetness() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Resource).unwrap();
        let mut palettes = 0;
        for entry in pack.entries() {
            if !entry.name.to_ascii_lowercase().ends_with(".ftb") {
                continue;
            }
            palettes += 1;
            let palette = Palette::parse(pack.bytes(entry)).unwrap();
            let wet = if entry.name.eq_ignore_ascii_case("uwswra1.ftb") {
                0.5
            } else {
                0.0
            };
            assert_eq!(palette.wetness, wet, "{}", entry.name);
            assert_eq!(palette.environment, 0.0, "{}", entry.name);
        }
        assert_eq!(palettes, 15);
    }

    /// The installed story level and outdoor arena keep terrain only on
    /// their flagged cells (skipped without the game).
    #[test]
    fn shipped_terrain_blocks() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Resource).unwrap();
        for (level, flagged) in [("exanima01", 682), ("arenao3", 240)] {
            let terrain = Terrain::parse(pack.get(&format!("{level}.rft")).unwrap()).unwrap();
            let scene =
                exa_formats::rfc::Scene::parse(pack.get(&format!("{level}.rfc")).unwrap()).unwrap();
            let g = scene.grid.unwrap();
            let rooms = Rooms::flood(&g);
            let blocks = grid_blocks(&terrain, &g, &rooms);
            let present: u32 = blocks
                .iter()
                .flat_map(|b| b.present.iter())
                .map(|r| r.count_ones())
                .sum();
            assert_eq!(present, flagged, "{level}");
            // No block on a closed cell (the rock between rooms).
            let x0 = (g.width as i64 - 8 * terrain.width as i64) / 2;
            let y0 = (g.height as i64 - 8 * terrain.height as i64) / 2;
            for (tile, b) in terrain.tiles.iter().zip(&blocks) {
                for by in 0..8 {
                    for bx in 0..8 {
                        if !b.has(bx, by) {
                            continue;
                        }
                        let x = x0 + 8 * tile.x as i64 + bx as i64;
                        let y = y0 + 8 * tile.y as i64 + by as i64;
                        let c = (y * g.width as i64 + x) as usize;
                        assert!(
                            g.edges[c] & crate::rooms::CELL_OPEN != 0,
                            "{level} ({x}, {y})"
                        );
                    }
                }
            }
        }
    }

    /// All 17 installed levels with a terrain have a tile grid, and none
    /// flags a cell in a tile slot its `.rft` leaves empty: the original's
    /// flat tile for such a slot, which Exarust leaves out, never appears
    /// (skipped without the game).
    #[test]
    fn shipped_levels_flag_no_empty_tile_slot() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(PackKind::Resource).unwrap();
        let side = TileBlocks::SIDE as i64;
        let mut levels = 0;
        for entry in pack.entries() {
            let Some(level) = entry.name.strip_suffix(".rft") else {
                continue;
            };
            levels += 1;
            let terrain = Terrain::parse(pack.bytes(entry)).unwrap();
            let rfc = pack.get(&format!("{level}.rfc")).unwrap();
            let g = exa_formats::rfc::Scene::parse(rfc).unwrap().grid;
            let g = g.unwrap_or_else(|| panic!("{level} has no tile grid"));
            let x0 = (g.width as i64 - side * terrain.width as i64) / 2;
            let y0 = (g.height as i64 - side * terrain.height as i64) / 2;
            for ty in 0..terrain.height {
                for tx in 0..terrain.width {
                    if terrain.tiles.iter().any(|t| (t.x, t.y) == (tx, ty)) {
                        continue;
                    }
                    for (bx, by) in (0..side).flat_map(|by| (0..side).map(move |bx| (bx, by))) {
                        let x = x0 + side * tx as i64 + bx;
                        let y = y0 + side * ty as i64 + by;
                        let flagged = x >= 0
                            && y >= 0
                            && g.cell(x as u32, y as u32)
                                .is_some_and(|c| g.edges[c] & CELL_TERRAIN != 0);
                        assert!(!flagged, "{level} ({x}, {y})");
                    }
                }
            }
        }
        assert_eq!(levels, 17);
    }
}

#[cfg(test)]
mod build_tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use bevy::prelude::World;
    use bevy::tasks::{AsyncComputeTaskPool, TaskPool};
    use exa_assets::{ModelCache, ModelLoader};

    /// A whole level's terrain builds through the tile worker on the async
    /// pool and ends with every tile spawned (skipped without the game).
    #[test]
    fn the_tile_worker_builds_a_shipped_level() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        AsyncComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mut queue = CommandQueue::default();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: true,
        };
        let (stats, pools, stream, detail) = {
            let mut commands = Commands::new(&mut queue, &world);
            let started = std::time::Instant::now();
            let mut build = TerrainBuild::begin(&mut loader, "exanima01", root, None).unwrap();
            let mut steps = 0;
            while !build.step(&mut commands, &mut loader, std::time::Duration::ZERO) {
                steps += 1;
                assert!(steps < 10_000, "terrain never finished");
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            eprintln!(
                "exanima01 terrain: {} steps, {:.2} s",
                steps,
                started.elapsed().as_secs_f32()
            );
            (
                build.stats().clone(),
                build.composite_pools().clone(),
                build.stream().clone(),
                build.detail_texture_loaded(),
            )
        };
        queue.apply(&mut world);
        assert_eq!(stats.tiles, 20);
        assert!(stats.plants > 0);
        // The original's composite pools are sized at the default texture
        // detail (8192): 21 sets at level 5 (1024²) and 43 at level 4
        // (512²). The detail texture is optional: the shipped install has no
        // `trndtl01d`, and no pass reads its handle either way.
        let _ = detail;
        assert_eq!((pools.high_level(), pools.low_level()), (5, 4));
        assert_eq!((pools.high_free(), pools.low_free()), (21, 43));
        // Every tile went through the reader, the worker and insertion.
        assert!(stream.tiles.iter().all(|t| t.state & 0x80 != 0));
        // Inserted, they stay in the frame hook's walk with nothing to do.
        assert!(
            stream
                .tiles
                .iter()
                .all(|t| t.state & 0x800 != 0 && t.action() == stream::TileAction::None)
        );
        assert!(world.get_resource::<ground::TerrainGround>().is_some());
        let ground = world.get_resource::<ground::TerrainGround>().unwrap();
        assert!(ground.height(0.0, 0.0).is_some());
        // The live driver has the tiles and every block, and a first frame
        // ranks them and bakes the blank ones.
        let mut live = world.remove_resource::<live::TerrainStreaming>().unwrap();
        assert_eq!(live.tiles().len(), 20);
        assert!(!live.blocks().is_empty());
        assert!(live.blocks().iter().all(|block| block.tile < 20));
        // The frame hook's inserts showed every spawned tile: none is left
        // hidden by an insert handled before its entity existed.
        for tile in live.tiles().iter().flatten() {
            assert_eq!(
                world.get::<Visibility>(tile.entity),
                Some(&Visibility::Visible),
                "a terrain tile stayed hidden"
            );
        }
        let visible = vec![true; live.blocks().len()];
        let distances = vec![None; live.blocks().len()];
        let tick = live.tick(Vec2::ZERO, &visible, &distances);
        assert!(tick.selected > 0);
        assert!(tick.changed);
        assert_eq!(live.frames(), 1);
        assert!(live.stream().all_inserted());
    }
}
