//! Terrain: one heightfield mesh per `.rft` tile, textured by layering the
//! palette materials its painted layers resolve to.
//!
//! Layers name a palette material or a blend rule; [`blend`] evaluates rules
//! per vertex into material weights. A tile keeps its four heaviest
//! materials, stacked in palette order like the original: the first is
//! opaque and each later one covers what is below by an "over" opacity,
//! sharpened by the material's blend mask. The opacities travel to the shader
//! in the vertex colour.

pub mod blend;
pub mod height;
pub mod vegetation;

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
use std::sync::Arc;

/// Materials a tile stacks (one vertex-colour channel each).
const SPLAT_LAYERS: usize = 4;
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
                )
                    .chain(),
            );
    }
}

/// The terrain's material: its layered textures, shaded with the original's
/// lighting (`exa_render::shading`) and its terrain gloss rule.
pub type TerrainMaterial = TerrainSplat;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct TerrainSplat {
    #[uniform(100)]
    pub params: SplatParams,
    #[texture(101)]
    #[sampler(102)]
    pub diffuse0: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    pub diffuse1: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    pub diffuse2: Option<Handle<Image>>,
    #[texture(107)]
    #[sampler(108)]
    pub diffuse3: Option<Handle<Image>>,
    /// Per layer: parallax height (red), gloss (green) and blend mask (blue)
    /// packed together, so a tile fits the per-stage texture limit.
    #[texture(109)]
    #[sampler(110)]
    pub surface0: Option<Handle<Image>>,
    #[texture(111)]
    #[sampler(112)]
    pub surface1: Option<Handle<Image>>,
    #[texture(113)]
    #[sampler(114)]
    pub surface2: Option<Handle<Image>>,
    #[texture(127)]
    #[sampler(128)]
    pub surface3: Option<Handle<Image>>,
    #[texture(115)]
    #[sampler(116)]
    pub normal0: Option<Handle<Image>>,
    #[texture(117)]
    #[sampler(118)]
    pub normal1: Option<Handle<Image>>,
    #[texture(119)]
    #[sampler(120)]
    pub normal2: Option<Handle<Image>>,
    #[texture(121)]
    #[sampler(122)]
    pub normal3: Option<Handle<Image>>,
    /// The voxel GI light volume, once it exists.
    #[texture(123, dimension = "3d")]
    #[sampler(124)]
    pub gi_volume: Option<Handle<Image>>,
    /// The environment cube.
    #[texture(125, dimension = "cube")]
    #[sampler(126)]
    pub environment: Option<Handle<Image>>,
    /// Light records ([`exa_render::lights`]); the default handle is the
    /// shared buffer.
    #[storage(130, read_only)]
    pub lights: Handle<bevy::render::storage::ShaderBuffer>,
    /// The cached cube shadow pool (`exa_render::cube_shadows`).
    #[texture(131, dimension = "cube_array", sample_type = "depth")]
    #[sampler(132, sampler_type = "comparison")]
    pub shadow_cubes: Handle<Image>,
}

impl Default for TerrainSplat {
    fn default() -> Self {
        Self {
            params: SplatParams::default(),
            diffuse0: None,
            diffuse1: None,
            diffuse2: None,
            diffuse3: None,
            surface0: None,
            surface1: None,
            surface2: None,
            surface3: None,
            normal0: None,
            normal1: None,
            normal2: None,
            normal3: None,
            gi_volume: None,
            environment: None,
            lights: Handle::default(),
            shadow_cubes: exa_render::cube_shadows::CUBE_POOL,
        }
    }
}

#[derive(ShaderType, Reflect, Debug, Clone, Copy, Default)]
pub struct SplatParams {
    /// Linear RGB multiplier per material; alpha is its cover strength.
    pub colour: [Vec4; SPLAT_LAYERS],
    /// Mask sharpness `s`, `t`, then 1 when the layer has a blend mask.
    pub blend: [Vec4; SPLAT_LAYERS],
    /// Mask sharpness `s`, `t` of the normal pass, normal-map strength (0
    /// without a normal map), gloss.
    pub normal: [Vec4; SPLAT_LAYERS],
    /// Texture repeats per metre per material.
    pub tiling: Vec4,
    /// Per layer: gloss factor (f10 × 1.5 × min(f9, 1)), then the parallax
    /// height's base, reference and scale (f20 + (B − f21)·f22).
    pub surface: [Vec4; SPLAT_LAYERS],
    /// The GI volume's lowest corner and size (metres).
    pub gi_origin: Vec4,
    pub gi_extent: Vec4,
    /// [`terrain_flags`].
    pub flags: u32,
    pub pad0: u32,
    pub pad1: u32,
    pub pad2: u32,
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
        m.params.gi_origin = textures.origin.extend(1.0);
        m.params.gi_extent = textures.extent.extend(0.0);
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
                // Grass and small plants are lit as if facing up; grass is smaller.
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
            .texture("plantset_01d", ColorSpace::Srgb)
            .map(|(t, _)| t);
        let normal_map_texture = loader
            .texture("plantset_01n", ColorSpace::Linear)
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
        let last = (n - 2) as f32;
        let block = |p: bevy::math::Vec2| {
            TileBlocks::of(
                (p.x.round().max(0.0) as usize).min(n - 1),
                (p.y.round().max(0.0) as usize).min(n - 1),
                n,
            )
        };
        let density = |p: bevy::math::Vec2| {
            if tile.is_hole(p.x.round() as usize, p.y.round() as usize) {
                return -1.0;
            }
            if let Some(b) = blocks {
                let (bx, by) = block(p);
                if !b.has(bx, by) {
                    return -1.0;
                }
            }
            let i = (p.x * last / (n - 1) as f32).round() as usize;
            let j = (p.y * last / (n - 1) as f32).round() as usize;
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
                    Mesh3d(loader.meshes.add(mesh)),
                    MeshMaterial3d(self.material.clone()),
                    Transform::IDENTITY,
                    Visibility::default(),
                    ChildOf(tile_entity),
                ))
                .id();
            if let Some(b) = blocks {
                let mid = chunk.iter().map(|g| g.at).sum::<bevy::math::Vec2>() / chunk.len() as f32;
                let (bx, by) = block(mid);
                if let Some(room) = b.room(bx, by) {
                    commands.entity(e).insert(InRoom(room));
                }
            }
        }
        grown
    }
}

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
/// frames short. Each part is one tile's heightfield and weights, one
/// palette material's textures (loaded the first time a tile uses it), or
/// spawning a built tile with its plants.
pub struct TerrainBuild {
    terrain: Terrain,
    palette: Palette,
    tree: RuleTree,
    noise: NoiseMaps,
    environment: Option<Handle<Image>>,
    /// Height and mask planes by texture name (lower case).
    planes: HashMap<String, Option<Arc<Plane>>>,
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
    next: usize,
    parent: Entity,
    stats: TerrainStats,
}

/// Bytes of textures one step makes at most (at least one material's): they
/// are uploaded to the GPU the frame after, and a level's palette at once
/// stalled a frame.
const UPLOAD_BUDGET: usize = 24 << 20;

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
        let palette = match resource
            .get(&format!("{}.ftb", terrain.palette))
            .map(Palette::parse)
        {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                loader.warn_once(format!("terrain palette {}: {e}", terrain.palette));
                return None;
            }
            None => {
                loader.warn_once(format!("terrain palette {} not found", terrain.palette));
                return None;
            }
        };
        let tree = RuleTree::new(&palette);
        let blocks = grid.map(|(grid, rooms)| grid_blocks(&terrain, grid, &rooms));
        let environment = loader.environment_map();
        let plants = Plants::load(loader, &palette);
        Some(Self {
            terrain,
            palette,
            tree,
            noise: NoiseMaps::default(),
            environment,
            planes: HashMap::new(),
            plants,
            textures: HashMap::new(),
            surfaces: HashMap::new(),
            uploaded: 0,
            built: None,
            blocks,
            next: 0,
            parent,
            stats: TerrainStats::default(),
        })
    }

    /// What the terrain spawned so far.
    pub fn stats(&self) -> &TerrainStats {
        &self.stats
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
        self.next >= self.terrain.tiles.len()
    }

    /// One part of the next tile: its heightfield, a material's textures,
    /// or the tile itself; `false` when it waits for a surface map.
    fn part(&mut self, commands: &mut Commands, loader: &mut ModelLoader<'_>) -> bool {
        let tile = &self.terrain.tiles[self.next];
        let blocks = self.blocks.as_ref().map(|b| &b[self.next]);
        // A tile without a block in a grid level is not built at all.
        if blocks.is_some_and(|b| !b.any()) {
            self.next += 1;
            return true;
        }
        let Some(built) = &mut self.built else {
            let game = loader.game;
            let (planes, palette) = (&mut self.planes, &self.palette);
            let mut height_layers = |used: &[usize]| height_layers(game, palette, planes, used);
            let built = build_tile(
                &self.terrain,
                tile,
                &self.palette,
                &self.tree,
                &self.noise,
                &mut height_layers,
                blocks,
            );
            self.stats.unresolved_layers += built.unresolved;
            self.stats.truncated_tiles += usize::from(built.truncated);
            // Its new materials' surface maps are packed meanwhile.
            for &m in &built.materials {
                if !self.textures.contains_key(&m) && !self.surfaces.contains_key(&m) {
                    let surface = Surface::start(loader.game, &self.palette.materials[m]);
                    self.surfaces.insert(m, surface);
                }
            }
            self.built = Some(built);
            return true;
        };
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
                .and_then(|name| loader.texture(name, ColorSpace::Srgb))
                .map(|(img, _)| img);
            let surface = Some(loader.images.add(surface));
            let normal = material
                .texture(TerrainTexture::Normal)
                .and_then(|name| loader.texture(name, ColorSpace::Linear))
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
            ..default()
        };
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
                0.0,
            );
            splat.params.tiling[slot] = look.tiling;
            splat.params.surface[slot] = Vec4::new(
                look.gloss,
                look.parallax[0],
                look.parallax[1],
                look.parallax[2],
            );
            match slot {
                0 => (splat.diffuse0, splat.surface0, splat.normal0) = (diffuse, surface, normal),
                1 => (splat.diffuse1, splat.surface1, splat.normal1) = (diffuse, surface, normal),
                2 => (splat.diffuse2, splat.surface2, splat.normal2) = (diffuse, surface, normal),
                _ => (splat.diffuse3, splat.surface3, splat.normal3) = (diffuse, surface, normal),
            }
        }
        let centre = self.terrain.tile_centre(tile);
        let tile_entity = commands
            .spawn((
                Name::new(format!("terrain {} {}", tile.x, tile.y)),
                PendingSplat(splat),
                Transform::from_xyz(centre[0], 0.0, centre[1]),
                Visibility::default(),
                ChildOf(self.parent),
            ))
            .id();
        let surface = (
            crate::gi_volume::GiTerrain,
            crate::shadow_links::Unwalled,
            exa_render::lights::ShadowBias(exa_render::shadow_bias::FLOOR),
        );
        let parts = std::mem::take(&mut built.parts);
        if blocks.is_none() {
            // Without a grid the tile is one mesh, on the tile's entity.
            if let Some((_, mesh)) = parts.into_iter().next() {
                commands
                    .entity(tile_entity)
                    .insert((Mesh3d(loader.meshes.add(mesh)), surface));
            }
        } else {
            for (room, mesh) in parts {
                let part = commands
                    .spawn((
                        Name::new(format!("terrain {} {} room {room:?}", tile.x, tile.y)),
                        Mesh3d(loader.meshes.add(mesh)),
                        TerrainPart,
                        surface,
                        Transform::IDENTITY,
                        Visibility::default(),
                        ChildOf(tile_entity),
                    ))
                    .id();
                if let Some(room) = room {
                    commands.entity(part).insert(InRoom(room));
                }
            }
        }
        commands
            .entity(tile_entity)
            .insert(crate::nav::view::Ground(Arc::new(
                crate::nav::GroundTile::new(&self.terrain, tile, built.heights.clone()),
            )));
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
        self.stats.tiles += 1;
        self.built = None;
        self.next += 1;
        true
    }
}

/// The height layers of the palette materials `used`, their height and mask
/// planes decoded once by name.
fn height_layers(
    game: &exa_assets::GameData,
    palette: &Palette,
    planes: &mut HashMap<String, Option<Arc<Plane>>>,
    used: &[usize],
) -> Vec<HeightLayer> {
    let mut plane = |name: Option<&str>| -> Option<Arc<Plane>> {
        let name = name?;
        planes
            .entry(name.to_ascii_lowercase())
            .or_insert_with(|| {
                let rfi = Rfi::parse(game.texture(name)?).ok()?;
                let rgba = rfi.decode_rgba8().ok()?;
                let (w, h) = (rfi.header.width as usize, rfi.header.height as usize);
                Plane::from_rgba(w, h, &rgba).map(Arc::new)
            })
            .clone()
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
        }
    }
}

struct BuiltTile {
    /// The tile's meshes, each with its room (one, roomless, without a
    /// grid); taken when the tile spawns.
    parts: Vec<(Option<u16>, Mesh)>,
    /// Displaced vertex heights (metres) and vegetation density per vertex.
    heights: Vec<f32>,
    density: Vec<f32>,
    /// Palette material indices, one per vertex-colour channel, ascending.
    materials: Vec<usize>,
    unresolved: usize,
    truncated: bool,
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
    let layers: Vec<(Option<usize>, &[u8])> = tile
        .layers
        .iter()
        .map(|l| (RuleTree::node(palette, &l.name), l.weights.as_slice()))
        .collect();
    let unresolved = layers.iter().filter(|l| l.0.is_none()).count();
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
    let colours: Vec<[f32; 4]> = (0..n * n)
        .map(|v| {
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
            std::array::from_fn(|k| alphas.get(k).copied().unwrap_or(0.0))
        })
        .collect();
    let normals = grid_normals(&positions, n);
    let heights: Vec<f32> = positions.iter().map(|p| p[1]).collect();

    // Quads (and, at the tile's edge, the skirt below them) go to the mesh
    // of their block's room; quads of absent blocks are not built.
    type Groups = Vec<(Option<u16>, Vec<u32>)>;
    fn group_of(room: Option<u16>, groups: &mut Groups) -> usize {
        match groups.iter().position(|g| g.0 == room) {
            Some(k) => k,
            None => {
                groups.push((room, Vec::new()));
                groups.len() - 1
            }
        }
    }
    let mut groups: Groups = Vec::new();
    let block_group = |i: usize, j: usize, groups: &mut Groups| match blocks {
        None => Some(group_of(None, groups)),
        Some(b) => {
            let (bx, by) = TileBlocks::of(i, j, n);
            b.has(bx, by).then(|| group_of(b.room(bx, by), groups))
        }
    };
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            if tile.is_hole(i, j) {
                continue;
            }
            let Some(g) = block_group(i, j, &mut groups) else {
                continue;
            };
            let a = (j * n + i) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            groups[g].1.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    // A skirt hanging from the tile's edge hides cracks where neighbouring
    // tiles lift a shared edge differently.
    let mut normals = normals;
    let mut colours = colours;
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
        colours.push(colours[v]);
    }
    for k in 0..edge.len() {
        let (a, b) = (edge[k] as u32, edge[(k + 1) % edge.len()] as u32);
        // The segment's block: the one holding its lower-left vertex.
        let (i, j) = (
            (a as usize % n).min(b as usize % n),
            (a as usize / n).min(b as usize / n),
        );
        let Some(g) = block_group(i, j, &mut groups) else {
            continue;
        };
        let (c, d) = (
            first_skirt + k as u32,
            first_skirt + ((k + 1) % edge.len()) as u32,
        );
        groups[g]
            .1
            .extend_from_slice(&[a, b, c, b, d, c, a, c, b, b, c, d]);
    }

    let vertices = TileVertices {
        positions,
        normals,
        uvs,
        colours,
    };
    let parts = match blocks {
        // Without a grid the whole tile is one mesh, as built.
        None => groups
            .pop()
            .map(|(room, indices)| (room, vertices.mesh(indices)))
            .into_iter()
            .collect(),
        Some(_) => groups
            .into_iter()
            .map(|(room, indices)| (room, vertices.part(&indices)))
            .collect(),
    };
    BuiltTile {
        parts,
        heights,
        density,
        materials: used,
        unresolved,
        truncated,
    }
}

/// A tile's vertex arrays, its skirt included.
struct TileVertices {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colours: Vec<[f32; 4]>,
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
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colours);
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
            colours: Vec::new(),
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
                    kept.colours.push(self.colours[v]);
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
            colours: vec![[1.0; 4]; 4],
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
}
