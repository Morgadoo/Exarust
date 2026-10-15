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
use exa_assets::texture::ColorSpace;
use exa_assets::{ModelLoader, PackKind};
use exa_formats::rfi::Rfi;
use exa_formats::terrain::{
    Palette, TILE_VERTICES, Terrain, TerrainMaterial as PaletteMaterial, TerrainTexture,
    TerrainTile,
};

use blend::{NoiseMaps, RuleTree, over_alphas};
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

/// A tile mesh waiting for its material, added by [`apply_splats`] which has
/// access to the material assets.
#[derive(Component)]
pub struct PendingSplat(pub TerrainSplat);

fn apply_splats(
    mut commands: Commands,
    pending: Query<(Entity, &PendingSplat)>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    for (e, splat) in &pending {
        let material = materials.add(splat.0.clone());
        commands
            .entity(e)
            .insert(MeshMaterial3d(material))
            .remove::<PendingSplat>();
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
    /// how many grew.
    fn spawn(
        &self,
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        terrain: &Terrain,
        tile: &TerrainTile,
        built: &BuiltTile,
        tile_entity: Entity,
    ) -> usize {
        let n = TILE_VERTICES;
        let last = (n - 2) as f32;
        let density = |p: bevy::math::Vec2| {
            if tile.is_hole(p.x.round() as usize, p.y.round() as usize) {
                return -1.0;
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
            commands.spawn((
                Name::new(format!("plants {}", k)),
                Mesh3d(loader.meshes.add(mesh)),
                MeshMaterial3d(self.material.clone()),
                Transform::IDENTITY,
                Visibility::default(),
                ChildOf(tile_entity),
            ));
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
/// `parent`.
pub fn spawn_terrain(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    level: &str,
    parent: Entity,
) -> Option<TerrainStats> {
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
    let environment = loader.environment_map();
    let noise = NoiseMaps::default();
    let game = loader.game;
    let mut planes: HashMap<String, Option<Arc<Plane>>> = HashMap::new();
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
    let mut height_layers = |used: &[usize]| -> Vec<HeightLayer> {
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
    };
    let plants = Plants::load(loader, &palette);
    let mut surfaces: HashMap<usize, Option<Handle<Image>>> = HashMap::new();
    let mut stats = TerrainStats::default();
    for tile in &terrain.tiles {
        let mut built = build_tile(&terrain, tile, &palette, &tree, &noise, &mut height_layers);
        stats.unresolved_layers += built.unresolved;
        stats.truncated_tiles += usize::from(built.truncated);
        let mut splat = TerrainSplat {
            environment: environment.clone(),
            ..default()
        };
        if splat.environment.is_some() {
            splat.params.flags |= terrain_flags::ENVIRONMENT;
        }
        for (slot, &m) in built.materials.iter().enumerate() {
            let material = &palette.materials[m];
            let diffuse = material
                .texture(TerrainTexture::Diffuse)
                .and_then(|name| loader.texture(name, ColorSpace::Srgb))
                .map(|(img, _)| img);
            let masked = material.texture(TerrainTexture::Mask).is_some();
            let surface = surfaces
                .entry(m)
                .or_insert_with(|| surface_texture(loader, material))
                .clone();
            let normal = material
                .texture(TerrainTexture::Normal)
                .and_then(|name| loader.texture(name, ColorSpace::Linear))
                .map(|(img, _)| img);
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
        let centre = terrain.tile_centre(tile);
        let tile_entity = commands
            .spawn((
                Name::new(format!("terrain {} {}", tile.x, tile.y)),
                Mesh3d(loader.meshes.add(built.mesh.take().expect("built once"))),
                PendingSplat(splat),
                crate::gi_volume::GiTerrain,
                crate::shadow_links::Unwalled,
                exa_render::lights::ShadowBias(exa_render::shadow_bias::FLOOR),
                Transform::from_xyz(centre[0], 0.0, centre[1]),
                Visibility::default(),
                ChildOf(parent),
            ))
            .id();
        if let Some(plants) = &plants {
            stats.plants += plants.spawn(commands, loader, &terrain, tile, &built, tile_entity);
        }
        stats.tiles += 1;
    }
    Some(stats)
}

/// Decodes a palette texture's base level to RGBA8 with its size.
fn decode(game: &exa_assets::GameData, name: Option<&str>) -> Option<(usize, usize, Vec<u8>)> {
    let rfi = Rfi::parse(game.texture(name?)?).ok()?;
    let rgba = rfi.decode_rgba8().ok()?;
    Some((rfi.header.width as usize, rfi.header.height as usize, rgba))
}

/// A palette material's parallax height (its bump slot, red), gloss (green)
/// and blend mask (blue) in one linear texture at the largest of their
/// sizes, with a box-filtered mip chain. Missing maps read as height ½,
/// gloss 0 and mask 1.
fn surface_texture(
    loader: &mut ModelLoader<'_>,
    material: &PaletteMaterial,
) -> Option<Handle<Image>> {
    let game = loader.game;
    let bump = decode(game, material.texture(TerrainTexture::Bump));
    let gloss = decode(game, material.texture(TerrainTexture::Gloss));
    let mask = decode(game, material.texture(TerrainTexture::Mask));
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
    Some(loader.images.add(image))
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
    /// Taken when the tile spawns.
    mesh: Option<Mesh>,
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

    let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            if tile.is_hole(i, j) {
                continue;
            }
            let a = (j * n + i) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
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
        let (c, d) = (
            first_skirt + k as u32,
            first_skirt + ((k + 1) % edge.len()) as u32,
        );
        indices.extend_from_slice(&[a, b, c, b, d, c, a, c, b, b, c, d]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    mesh.insert_indices(Indices::U32(indices));
    BuiltTile {
        mesh: Some(mesh),
        heights,
        density,
        materials: used,
        unresolved,
        truncated,
    }
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
}
