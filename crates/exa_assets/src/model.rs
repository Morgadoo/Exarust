//! Chunk-tree models to Bevy meshes and materials, cached by name so each
//! model, material and texture is decoded and uploaded once.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_formats::material::{Material, TextureSlot};
use exa_formats::rfc::{Mesh as RfcMesh, NodeBody, Scene};
use exa_formats::rfi::Rfi;
use exa_rpk::{Archive, GameData, PackKind};

use crate::texture::{ColorSpace, has_alpha, rfi_to_image};

/// A decoded model ready to spawn any number of times.
#[derive(Debug, Clone)]
pub struct Model {
    pub nodes: Vec<ModelNode>,
}

#[derive(Debug, Clone)]
pub struct ModelNode {
    pub name: String,
    /// Index into [`Model::nodes`].
    pub parent: Option<usize>,
    pub transform: Transform,
    pub parts: Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,
    pub light: Option<exa_formats::rfc::Light>,
}

impl Model {
    pub fn roots(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].parent.is_none())
    }
}

/// Converts a stored 3×4 row-vector matrix (rotation rows, translation row)
/// into a Bevy transform.
pub fn transform_from_rows(t: &[[f32; 3]; 4]) -> Transform {
    let m = Mat4::from_cols(
        Vec3::from(t[0]).extend(0.0),
        Vec3::from(t[1]).extend(0.0),
        Vec3::from(t[2]).extend(0.0),
        Vec3::from(t[3]).extend(1.0),
    );
    Transform::from_matrix(m)
}

/// A loaded texture and whether it carries alpha.
type CachedTexture = Option<(Handle<Image>, bool)>;

#[derive(Resource, Default)]
pub struct ModelCache {
    models: HashMap<String, Option<Arc<Model>>>,
    textures: HashMap<(String, ColorSpace), CachedTexture>,
    materials: HashMap<String, Handle<StandardMaterial>>,
    warned: HashSet<String>,
}

/// Everything conversion needs from the world.
pub struct ModelLoader<'a> {
    pub game: &'a GameData,
    pub cache: &'a mut ModelCache,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub images: &'a mut Assets<Image>,
    pub bc_supported: bool,
}

pub fn bc_supported(support: Option<&CompressedImageFormatSupport>) -> bool {
    support.is_some_and(|s| s.0.contains(CompressedImageFormats::BC))
}

impl ModelLoader<'_> {
    /// Logs a warning once per distinct message.
    pub fn warn_once(&mut self, msg: String) {
        if self.cache.warned.insert(msg.clone()) {
            warn!("{msg}");
        }
    }

    pub fn model(&mut self, pack: PackKind, entry: &str) -> Option<Arc<Model>> {
        let key = format!("{}/{}", pack.file_stem(), entry.to_ascii_lowercase());
        if let Some(cached) = self.cache.models.get(&key) {
            return cached.clone();
        }
        let model = self.load(pack, entry).map(Arc::new);
        self.cache.models.insert(key, model.clone());
        model
    }

    fn load(&mut self, pack: PackKind, entry: &str) -> Option<Model> {
        let Some(bytes) = self.game.pack(pack).and_then(|p| p.get(entry)) else {
            self.warn_once(format!("model {}/{entry} not found", pack.file_stem()));
            return None;
        };
        let scene = match Scene::parse(bytes) {
            Ok(s) => s,
            Err(e) => {
                self.warn_once(format!("model {}/{entry}: {e}", pack.file_stem()));
                return None;
            }
        };
        let mut nodes = Vec::with_capacity(scene.nodes.len());
        for node in &scene.nodes {
            let parts = match &node.body {
                NodeBody::Mesh(mesh) => self.parts(entry, &scene, mesh),
                _ => Vec::new(),
            };
            nodes.push(ModelNode {
                name: node.name.clone(),
                parent: node
                    .parent
                    .filter(|&p| p >= 1 && p <= scene.nodes.len())
                    .map(|p| p - 1),
                transform: transform_from_rows(&node.transform),
                parts,
                light: match &node.body {
                    NodeBody::Light(l) => Some(*l),
                    _ => None,
                },
            });
        }
        Some(Model { nodes })
    }

    fn parts(
        &mut self,
        entry: &str,
        scene: &Scene,
        mesh: &RfcMesh,
    ) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
        let mut out = Vec::with_capacity(mesh.groups.len());
        for group in &mesh.groups {
            let start = group.first_index as usize;
            let indices = &mesh.indices[start..start + group.index_count as usize];
            let Some(bevy_mesh) = group_mesh(mesh, indices) else {
                continue;
            };
            // `_null` marks geometry the original never draws.
            if group.material.eq_ignore_ascii_case("_null") {
                continue;
            }
            let material = self.material(entry, scene, &group.material);
            out.push((self.meshes.add(bevy_mesh), material));
        }
        out
    }

    fn material(&mut self, entry: &str, scene: &Scene, name: &str) -> Handle<StandardMaterial> {
        let local = scene.material(name).cloned();
        let key = if local.is_some() {
            format!(
                "{}#{}",
                entry.to_ascii_lowercase(),
                name.to_ascii_lowercase()
            )
        } else {
            format!("lib#{}", name.to_ascii_lowercase())
        };
        if let Some(h) = self.cache.materials.get(&key) {
            return h.clone();
        }
        // `&` names are resolved at run time by the item system; their texture
        // set follows from the name (see `apparel_texture_key`).
        if let Some(texture_key) = apparel_texture_key(name) {
            let handle = self.apparel_material(&texture_key);
            self.cache.materials.insert(key, handle.clone());
            return handle;
        }
        let source = local.or_else(|| library_material(self.game, name));
        let mut mat = StandardMaterial {
            perceptual_roughness: 0.85,
            metallic: 0.0,
            reflectance: 0.3,
            ..default()
        };
        match source {
            Some(m) => {
                let [r, g, b, _] = m.diffuse.map(|v| v.clamp(0.0, 1.0));
                mat.base_color = Color::linear_rgb(r, g, b);
                let spec = (m.specular[0] + m.specular[1] + m.specular[2]) / 3.0;
                mat.reflectance = (spec * 0.5).clamp(0.0, 0.5);
                if let Some(t) = m.texture(TextureSlot::Diffuse)
                    && let Some((img, alpha)) = self.texture(t, ColorSpace::Srgb)
                {
                    mat.base_color_texture = Some(img);
                    if alpha {
                        mat.alpha_mode = AlphaMode::Mask(0.5);
                        mat.double_sided = true;
                        mat.cull_mode = None;
                    }
                }
                if let Some(t) = m.texture(TextureSlot::Normal)
                    && let Some((img, _)) = self.texture(t, ColorSpace::Linear)
                {
                    mat.normal_map_texture = Some(img);
                }
            }
            None => {
                self.warn_once(format!("material {name:?} not found"));
                mat.base_color = Color::srgb(0.6, 0.6, 0.6);
            }
        }
        let handle = self.materials.add(mat);
        self.cache.materials.insert(key, handle.clone());
        handle
    }

    fn apparel_material(&mut self, texture_key: &str) -> Handle<StandardMaterial> {
        let mut mat = StandardMaterial {
            perceptual_roughness: 0.8,
            metallic: 0.0,
            reflectance: 0.3,
            ..default()
        };
        match self.texture(&format!("{texture_key}d"), ColorSpace::Srgb) {
            Some((img, alpha)) => {
                mat.base_color_texture = Some(img);
                if alpha {
                    mat.alpha_mode = AlphaMode::Mask(0.5);
                }
            }
            None => mat.base_color = Color::srgb(0.5, 0.45, 0.4),
        }
        if let Some((img, _)) = self.texture(&format!("{texture_key}n"), ColorSpace::Linear) {
            mat.normal_map_texture = Some(img);
        }
        self.materials.add(mat)
    }

    /// Image handle and whether it has alpha.
    pub fn texture(&mut self, name: &str, space: ColorSpace) -> CachedTexture {
        let key = (name.to_ascii_lowercase(), space);
        if let Some(cached) = self.cache.textures.get(&key) {
            return cached.clone();
        }
        let result = match self.game.texture(name) {
            None => Err(format!("texture {name:?} not found")),
            Some(bytes) => Rfi::parse(bytes)
                .and_then(|rfi| {
                    let alpha = has_alpha(&rfi);
                    rfi_to_image(&rfi, space, self.bc_supported).map(|img| (img, alpha))
                })
                .map_err(|e| format!("texture {name:?}: {e}")),
        };
        let value = match result {
            Ok((img, alpha)) => Some((self.images.add(img), alpha)),
            Err(msg) => {
                self.warn_once(msg);
                None
            }
        };
        self.cache.textures.insert(key, value.clone());
        value
    }
}

fn library_material(game: &GameData, name: &str) -> Option<Material> {
    let libs = game
        .project()
        .material_libraries
        .iter()
        .map(String::as_str)
        .chain(["humanbase.rml"]);
    for lib in libs {
        if let Some(bytes) = game.resource_file(lib)
            && let Ok(archive) = Archive::parse(bytes)
            && let Some(payload) = archive.get(name)
            && let Ok(m) = Material::parse(payload)
        {
            return Some(m);
        }
    }
    None
}

/// A Bevy mesh holding only the vertices one material group uses.
fn group_mesh(mesh: &RfcMesh, indices: &[u32]) -> Option<Mesh> {
    if indices.is_empty() {
        return None;
    }
    let mut remap = HashMap::with_capacity(indices.len());
    let mut used = Vec::new();
    let local: Vec<u32> = indices
        .iter()
        .map(|&i| {
            *remap.entry(i).or_insert_with(|| {
                used.push(i as usize);
                (used.len() - 1) as u32
            })
        })
        .collect();
    let positions: Vec<[f32; 3]> = used.iter().map(|&i| mesh.positions[i]).collect();
    let normals: Vec<[f32; 3]> = used.iter().map(|&i| mesh.normals[i]).collect();
    // Kept in the main world too: static colliders are built from mesh data.
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(local));
    if let Some(uvs) = &mesh.uvs {
        let uv: Vec<[f32; 2]> = used.iter().map(|&i| uvs[i]).collect();
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        // Tangents are only needed for normal mapping; failures leave the mesh usable.
        let _ = m.generate_tangents();
    }
    Some(m)
}

/// Texture-name stem for a run-time material `&<model><set><n>`: the model
/// name's base padded with `_` to eight characters, its variant, then the
/// texture set (`&doublet_ba1` → `doublet_ba`, completed with `d`, `n`, `m1`).
pub fn apparel_texture_key(material: &str) -> Option<String> {
    let rest = material.strip_prefix('&')?;
    let rest = rest.trim_end_matches(|c: char| c.is_ascii_digit());
    let (model, set) = rest.split_at(rest.len().checked_sub(1)?);
    let (base, variant) = model.rsplit_once('_')?;
    if base.is_empty() || variant.is_empty() {
        return None;
    }
    let base = base.trim_end_matches('_');
    Some(format!("{base:_<8}{variant}{set}").to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apparel_keys() {
        assert_eq!(
            apparel_texture_key("&doublet_ba1").as_deref(),
            Some("doublet_ba")
        );
        assert_eq!(
            apparel_texture_key("&armet___ba1").as_deref(),
            Some("armet___ba")
        );
        assert_eq!(
            apparel_texture_key("&helm_ba2").as_deref(),
            Some("helm____ba")
        );
        assert_eq!(apparel_texture_key("skinhuman"), None);
    }
}
