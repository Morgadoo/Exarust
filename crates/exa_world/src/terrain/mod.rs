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

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use exa_assets::texture::ColorSpace;
use exa_assets::{ModelLoader, PackKind};
use exa_formats::terrain::{
    Palette, TILE_VERTICES, Terrain, TerrainMaterial as PaletteMaterial, TerrainTexture,
    TerrainTile,
};

use blend::{NoiseMaps, RuleTree, over_alphas};

/// Materials a tile stacks (one vertex-colour channel each).
const SPLAT_LAYERS: usize = 4;
/// Texture coordinates advance this much per metre before a material's
/// tiling factor (4 per 48 m tile).
const UV_PER_METRE: f32 = 4.0 / 48.0;
/// Diffuse colour is texture × tint × this.
const TINT_GAIN: f32 = 1.5;
/// Materials whose share of a tile is below this are dropped.
const MIN_TILE_SHARE: f32 = 1.0 / 510.0;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "terrain.wesl");
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(PostUpdate, apply_splats);
    }
}

pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainSplat>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
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
    #[texture(109)]
    #[sampler(110)]
    pub mask1: Option<Handle<Image>>,
    #[texture(111)]
    #[sampler(112)]
    pub mask2: Option<Handle<Image>>,
    #[texture(113)]
    #[sampler(114)]
    pub mask3: Option<Handle<Image>>,
}

#[derive(ShaderType, Reflect, Debug, Clone, Copy, Default)]
pub struct SplatParams {
    /// Linear RGB multiplier per material; alpha is its cover strength.
    pub colour: [Vec4; SPLAT_LAYERS],
    /// Mask sharpness `s`, `t`, then 1 when the layer has a blend mask.
    pub blend: [Vec4; SPLAT_LAYERS],
    /// Texture repeats per metre per material.
    pub tiling: Vec4,
}

fn shader() -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(embedded_path!("terrain.wesl")).with_source("embedded"),
    )
}

impl MaterialExtension for TerrainSplat {
    fn fragment_shader() -> ShaderRef {
        shader()
    }

    fn deferred_fragment_shader() -> ShaderRef {
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
        let material = materials.add(TerrainMaterial {
            base: StandardMaterial {
                perceptual_roughness: 0.85,
                reflectance: 0.2,
                ..default()
            },
            extension: splat.0.clone(),
        });
        commands
            .entity(e)
            .insert(MeshMaterial3d(material))
            .remove::<PendingSplat>();
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
    let noise = NoiseMaps::default();
    let mut stats = TerrainStats::default();
    for tile in &terrain.tiles {
        let built = build_tile(&terrain, tile, &palette, &tree, &noise);
        stats.unresolved_layers += built.unresolved;
        stats.truncated_tiles += usize::from(built.truncated);
        let mut splat = TerrainSplat::default();
        for (slot, &m) in built.materials.iter().enumerate() {
            let material = &palette.materials[m];
            let diffuse = material
                .texture(TerrainTexture::Diffuse)
                .and_then(|name| loader.texture(name, ColorSpace::Srgb))
                .map(|(img, _)| img);
            let mask = material
                .texture(TerrainTexture::Mask)
                .and_then(|name| loader.texture(name, ColorSpace::Linear))
                .map(|(img, _)| img);
            let look = Look::of(material);
            splat.params.colour[slot] = look.colour;
            splat.params.blend[slot] = Vec4::new(
                look.sharpness[0],
                look.sharpness[1],
                if mask.is_some() { 1.0 } else { 0.0 },
                0.0,
            );
            splat.params.tiling[slot] = look.tiling;
            match slot {
                0 => splat.diffuse0 = diffuse,
                1 => (splat.diffuse1, splat.mask1) = (diffuse, mask),
                2 => (splat.diffuse2, splat.mask2) = (diffuse, mask),
                _ => (splat.diffuse3, splat.mask3) = (diffuse, mask),
            }
        }
        let centre = terrain.tile_centre(tile);
        commands.spawn((
            Name::new(format!("terrain {} {}", tile.x, tile.y)),
            Mesh3d(loader.meshes.add(built.mesh)),
            PendingSplat(splat),
            Transform::from_xyz(centre[0], 0.0, centre[1]),
            Visibility::default(),
            ChildOf(parent),
        ));
        stats.tiles += 1;
    }
    Some(stats)
}

/// Shading values of a palette material, from its parameter block.
struct Look {
    colour: Vec4,
    sharpness: [f32; 2],
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
            tiling: tile * UV_PER_METRE,
        }
    }
}

struct BuiltTile {
    mesh: Mesh,
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

    let colours: Vec<[f32; 4]> = (0..n * n)
        .map(|v| {
            let w = &weights[v * m..(v + 1) * m];
            let kept: Vec<f32> = used.iter().map(|&i| w[i]).collect();
            let alphas = over_alphas(&kept);
            std::array::from_fn(|k| alphas.get(k).copied().unwrap_or(0.0))
        })
        .collect();

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
        mesh,
        materials: used,
        unresolved,
        truncated,
    }
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
        assert!((look.tiling - 2.0 / 12.0).abs() < 1e-6);
    }
}
