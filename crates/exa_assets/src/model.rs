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
use exa_formats::rsq::bone_index;
use exa_rpk::{Archive, GameData, PackKind};

use crate::paint::{PaintLayer, Texels, layer_material, library, normal_library, paint};
use crate::skin::{Joint, VertexSkin, auto_skin};
use crate::texture::{ColorSpace, has_alpha, rfi_to_image};
use exa_formats::material::flags as material_flags;
use exa_formats::objdb::Appearance;
use exa_render::material::{ExaMaterial, ExaParams, flags, linear_rgb};

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
    pub parts: Vec<Part>,
    /// Material name of each part, as stored in the file.
    pub part_materials: Vec<String>,
    /// Bone whose space the parts are in, when their meshes carry skin
    /// weights (joint indices are bone numbers).
    pub skin_bone: Option<u16>,
    pub light: Option<exa_formats::rfc::Light>,
    /// Size of the fire this node becomes (torches 0.2, braziers 0.41).
    pub fire: Option<f32>,
    /// Bounding sphere in the node's space (metres).
    pub bounds: (Vec3, f32),
    /// The mesh's GI sample points in the node's space (metres), which mark
    /// the voxels it fills.
    pub gi_points: Arc<Vec<Vec3>>,
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

/// The human body model; its parts are skinned and apparel follows them.
pub const BODY_MODEL: &str = "humanbase.rfc";

/// Whether a `Resource.rpk` entry is a race's body model (`humanbase.rfc`,
/// `skelbase.rfc`, `golembase.rfc`, …).
pub fn is_body_model(entry: &str) -> bool {
    entry.to_ascii_lowercase().ends_with("base.rfc")
}

/// A mesh part: its mesh and material.
pub type Part = (Handle<Mesh>, Handle<ExaMaterial>);

/// A loaded texture and whether it carries alpha.
type CachedTexture = Option<(Handle<Image>, bool)>;

#[derive(Resource, Default)]
pub struct ModelCache {
    models: HashMap<String, Option<Arc<Model>>>,
    textures: HashMap<(String, ColorSpace), CachedTexture>,
    materials: HashMap<String, Handle<ExaMaterial>>,
    /// Textures decoded on the CPU, for painting.
    texels: HashMap<String, Option<Arc<Texels>>>,
    /// Painted apparel materials by texture stem and appearance.
    painted: HashMap<(String, Appearance), Handle<ExaMaterial>>,
    /// The environment cube, once looked for.
    environment: Option<Option<Handle<Image>>>,
    /// The body's skinned vertices per bone (part space, metres), for
    /// apparel to take its weights from.
    body_skin: HashMap<u16, Arc<Vec<(Vec3, VertexSkin)>>>,
    warned: HashSet<String>,
}

/// Everything conversion needs from the world.
pub struct ModelLoader<'a> {
    pub game: &'a GameData,
    pub cache: &'a mut ModelCache,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<ExaMaterial>,
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

    pub(crate) fn cached_model(&self, key: &str) -> Option<Option<Arc<Model>>> {
        self.cache.models.get(key).cloned()
    }

    pub(crate) fn cache_model(&mut self, key: String, model: Option<Arc<Model>>) {
        self.cache.models.insert(key, model);
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
        // Apparel takes its weights from the body, which must be known first.
        if pack == PackKind::Characters && self.cache.body_skin.is_empty() {
            self.model(PackKind::Resource, BODY_MODEL);
        }
        let mut nodes = Vec::with_capacity(scene.nodes.len());
        for (index, node) in scene.nodes.iter().enumerate() {
            let skin = match &node.body {
                NodeBody::Mesh(mesh)
                    if pack == PackKind::Characters
                        || (pack == PackKind::Resource && is_body_model(entry)) =>
                {
                    self.node_skin(&scene, index, mesh, pack, entry)
                }
                _ => None,
            };
            let skin_bone = skin.as_ref().map(|(bone, _)| *bone);
            let (parts, part_materials) = match &node.body {
                NodeBody::Mesh(mesh) => self
                    .parts(
                        entry,
                        &scene,
                        mesh,
                        skin.as_ref().map(|(_, s)| s.as_slice()),
                    )
                    .into_iter()
                    .unzip(),
                _ => (Vec::new(), Vec::new()),
            };
            nodes.push(ModelNode {
                name: node.name.clone(),
                parent: node
                    .parent
                    .filter(|&p| p >= 1 && p <= scene.nodes.len())
                    .map(|p| p - 1),
                transform: transform_from_rows(&node.transform),
                parts,
                part_materials,
                skin_bone,
                bounds: (Vec3::from(node.bounds_centre), node.bounds_radius),
                gi_points: Arc::new(match &node.body {
                    NodeBody::Mesh(mesh) if skin_bone.is_none() => {
                        mesh.gi_points.iter().map(|&p| Vec3::from(p)).collect()
                    }
                    _ => Vec::new(),
                }),
                fire: match &node.body {
                    NodeBody::Emitter { kind, size } if *kind == exa_formats::rfc::FIRE => {
                        Some(*size)
                    }
                    _ => None,
                },
                light: match &node.body {
                    NodeBody::Light(l) => Some(*l),
                    _ => None,
                },
            });
        }
        Some(Model { nodes })
    }

    /// Skin weights of a node's mesh: worked out from the seams for body
    /// parts (nodes named after a bone with bone neighbours), taken from the
    /// nearest body vertex for apparel. Remembers the human body's weights,
    /// which apparel is fitted to.
    fn node_skin(
        &mut self,
        scene: &Scene,
        index: usize,
        mesh: &RfcMesh,
        pack: PackKind,
        entry: &str,
    ) -> Option<(u16, Vec<VertexSkin>)> {
        let node = &scene.nodes[index];
        let bone = bone_index(&node.name)? as u16;
        if pack == PackKind::Characters {
            let body = self.cache.body_skin.get(&bone)?.clone();
            let skin = mesh
                .positions
                .iter()
                .map(|p| {
                    let p = Vec3::from(*p);
                    body.iter()
                        .min_by(|a, b| a.0.distance_squared(p).total_cmp(&b.0.distance_squared(p)))
                        .map_or([(bone, 1.0), (bone, 0.0), (bone, 0.0), (bone, 0.0)], |b| {
                            b.1
                        })
                })
                .collect();
            return Some((bone, skin));
        }
        let bone_of = |i: usize| bone_index(&scene.nodes[i].name).map(|b| b as u16);
        let mut joints = Vec::new();
        if let Some(parent) = node.parent.and_then(|p| p.checked_sub(1)).and_then(bone_of) {
            joints.push(Joint {
                bone: parent,
                pivot: Vec3::ZERO,
            });
        }
        for (i, child) in scene.nodes.iter().enumerate() {
            if child.parent == Some(index + 1)
                && let Some(b) = bone_of(i)
            {
                joints.push(Joint {
                    bone: b,
                    pivot: Vec3::from(child.transform[3]),
                });
            }
        }
        if joints.is_empty() || mesh.triangle_flags.is_none() {
            return None;
        }
        let skin = auto_skin(mesh, bone, &joints);
        if !entry.eq_ignore_ascii_case(BODY_MODEL) {
            return Some((bone, skin));
        }
        let reference = mesh
            .positions
            .iter()
            .map(|p| Vec3::from(*p))
            .zip(skin.iter().copied());
        self.cache
            .body_skin
            .entry(bone)
            .or_insert_with(|| Arc::new(reference.collect()));
        Some((bone, skin))
    }

    fn parts(
        &mut self,
        entry: &str,
        scene: &Scene,
        mesh: &RfcMesh,
        skin: Option<&[VertexSkin]>,
    ) -> Vec<(Part, String)> {
        let mut out = Vec::with_capacity(mesh.groups.len());
        for group in &mesh.groups {
            let start = group.first_index as usize;
            let indices = &mesh.indices[start..start + group.index_count as usize];
            let Some(bevy_mesh) = group_mesh(mesh, indices, skin) else {
                continue;
            };
            // `_null` marks geometry the original never draws.
            if group.material.eq_ignore_ascii_case("_null") {
                continue;
            }
            let material = self.material(entry, scene, &group.material);
            out.push((
                (self.meshes.add(bevy_mesh), material),
                group.material.clone(),
            ));
        }
        out
    }

    fn material(&mut self, entry: &str, scene: &Scene, name: &str) -> Handle<ExaMaterial> {
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
        let mat = match source {
            Some(m) => self.exa_material(&m),
            None => {
                self.warn_once(format!("material {name:?} not found"));
                ExaMaterial {
                    params: ExaParams {
                        diffuse: Vec4::new(0.32, 0.32, 0.32, 1.0),
                        ..default()
                    },
                    ..default()
                }
            }
        };
        let handle = self.materials.add(mat);
        self.cache.materials.insert(key, handle.clone());
        handle
    }

    /// A material shaded like the original's from its library record.
    fn exa_material(&mut self, m: &Material) -> ExaMaterial {
        let rgb = |c: [f32; 4]| linear_rgb([c[0], c[1], c[2]]);
        // Legacy materials brighten their diffuse colour.
        let mut diffuse = rgb(m.diffuse);
        if m.kind == 0 {
            diffuse *= 1.5;
        }
        let packed = m.kind & 1 != 0;
        // No specular without the material's specular bit; PBR materials
        // with a specular map triple it.
        let mut specular = rgb(m.specular) * m.specular_level;
        if m.flags & material_flags::SPECULAR == 0 {
            specular = Vec3::ZERO;
        } else if packed && m.texture(TextureSlot::Specular).is_some() {
            specular *= 3.0;
        }
        let mut shading = 0;
        if m.flags & material_flags::LIT != 0 {
            shading |= flags::LIT;
        }
        if m.flags & material_flags::TWO_SIDED != 0 {
            shading |= flags::TWO_SIDED;
        }
        if packed {
            shading |= flags::PACKED_SPECULAR;
        }
        let mut mat = ExaMaterial {
            params: ExaParams {
                diffuse: diffuse.extend(m.opacity.clamp(0.0, 1.0)),
                specular: specular.extend(m.glossiness.clamp(0.01, 1.0)),
                emissive: rgb(m.emissive).extend(m.emissive[3]),
                flags: shading,
                ..default()
            },
            ..default()
        };
        let mut cut_out = false;
        if let Some(t) = m.texture(TextureSlot::Diffuse)
            && let Some((img, alpha)) = self.texture(t, ColorSpace::Srgb)
        {
            mat.diffuse_map = Some(img);
            cut_out = alpha;
        }
        if let Some(t) = m.texture(TextureSlot::Normal) {
            mat.normal_map = self.texture(t, ColorSpace::Linear).map(|t| t.0);
        }
        if let Some(t) = m.texture(TextureSlot::Specular) {
            let space = if packed {
                ColorSpace::Linear
            } else {
                ColorSpace::Srgb
            };
            mat.specular_map = self.texture(t, space).map(|t| t.0);
        }
        if let Some(t) = m.texture(TextureSlot::Emissive) {
            mat.emissive_map = self.texture(t, ColorSpace::Srgb).map(|t| t.0);
        }
        if let Some(t) = m.texture(TextureSlot::Opacity) {
            mat.opacity_map = self.texture(t, ColorSpace::Linear).map(|t| t.0);
        }
        // Dirt maps only act through vertex colours, which level meshes lack.
        if let Some(t) = m.texture(TextureSlot::Gloss) {
            mat.gloss_map = self.texture(t, ColorSpace::Linear).map(|t| t.0);
        }
        // Parallax: the height layer's depth as a fixed number of texels of
        // the height map; displacement shadows when the record asks.
        if let Some(layer) = m.layer(TextureSlot::Height)
            && layer.amount > 0.0
            && mat.normal_map.is_some()
            && let Some((img, _)) = self.texture(&layer.texture, ColorSpace::Linear)
            && let Some(size) = self.images.get(&img).map(|i| i.size())
            && size.x > 1
            && size.y > 1
        {
            let (w, h) = (size.x as f32, size.y as f32);
            let depth = (50.0 * layer.amount).round().min(65535.0);
            let y = (depth * 2048.0 / h).round();
            let x = (y * h / w).round();
            mat.params.displacement = Vec4::new(
                x * PARALLAX_UNIT,
                y * PARALLAX_UNIT,
                m.displacement_step * 0.004,
                m.displacement_bias * 0.05,
            );
            mat.params.flags |= flags::HEIGHT_MAP;
            if m.extra & DISPLACEMENT_SHADOWS != 0 {
                mat.params.flags |= flags::DISPLACEMENT_SHADOWS;
            }
            mat.height_map = Some(img);
        }
        mat.alpha_mode = blend_mode(m.class, cut_out || mat.opacity_map.is_some());
        if mat.alpha_mode == AlphaMode::Premultiplied {
            let a = mat.params.diffuse.w;
            mat.params.diffuse = (mat.params.diffuse.truncate() * a).extend(a);
        }
        mat.environment = self.environment_map();
        mat.with_map_flags()
    }

    fn apparel_material(&mut self, texture_key: &str) -> Handle<ExaMaterial> {
        let mut mat = ExaMaterial {
            environment: self.environment_map(),
            ..default()
        };
        match self.texture(&format!("{texture_key}d"), ColorSpace::Srgb) {
            Some((img, alpha)) => {
                mat.diffuse_map = Some(img);
                if alpha {
                    mat.alpha_mode = AlphaMode::Mask(0.5);
                }
            }
            None => mat.params.diffuse = Vec4::new(0.21, 0.18, 0.13, 1.0),
        }
        mat.normal_map = self
            .texture(&format!("{texture_key}n"), ColorSpace::Linear)
            .map(|t| t.0);
        self.materials.add(mat.with_map_flags())
    }

    /// A texture decoded to RGBA8 on the CPU, cached by name.
    pub fn texels(&mut self, name: &str) -> Option<Arc<Texels>> {
        let key = name.to_ascii_lowercase();
        if let Some(t) = self.cache.texels.get(&key) {
            return t.clone();
        }
        let decoded = self.game.texture(name).and_then(|b| {
            let rfi = Rfi::parse(b).ok()?;
            let rgba = rfi.decode_rgba8().ok()?;
            Some(Arc::new(Texels {
                width: rfi.header.width as usize,
                height: rfi.header.height as usize,
                rgba,
            }))
        });
        if decoded.is_none() {
            self.warn_once(format!(
                "texture {name:?} could not be decoded for painting"
            ));
        }
        self.cache.texels.insert(key, decoded.clone());
        decoded
    }

    /// The material of an apparel texture set (`stem`, e.g. `doublet_aa`)
    /// painted with an item's appearance, as the original paints equipped
    /// items. Falls back to the unpainted material when the shade map is
    /// missing.
    pub fn painted_apparel(&mut self, stem: &str, appearance: &Appearance) -> Handle<ExaMaterial> {
        let key = (stem.to_ascii_lowercase(), *appearance);
        if let Some(h) = self.cache.painted.get(&key) {
            return h.clone();
        }
        let handle = match self.texels(&format!("{stem}d")) {
            Some(shade) => {
                let mut owned: Vec<OwnedLayer> = Vec::new();
                for layer in appearance.used_layers() {
                    // Items and recipes may name masks a texture set lacks.
                    let mask_name = format!("{stem}m{}", layer.mask);
                    if self.game.texture(&mask_name).is_none() {
                        continue;
                    }
                    let Some(mask) = self.texels(&mask_name) else {
                        continue;
                    };
                    let material = layer_material(layer);
                    let lib = library(material).iter().map(|n| self.texels(n)).collect();
                    let normals = normal_library(material)
                        .iter()
                        .map(|n| self.texels(n))
                        .collect();
                    owned.push((mask, material, layer.colour, lib, normals));
                }
                let layers: Vec<PaintLayer<'_>> = owned
                    .iter()
                    .map(|(mask, material, colour, lib, normals)| PaintLayer {
                        mask,
                        material: *material,
                        colour: *colour,
                        library: lib.iter().map(|t| t.as_deref()).collect(),
                        normals: normals.iter().map(|t| t.as_deref()).collect(),
                    })
                    .collect();
                let item_normal = self.texels(&format!("{stem}n"));
                let painted = paint(&shade, item_normal.as_deref(), &layers, appearance);
                let mut rgba = painted.colour;
                // Keep the cut-outs of the shade map.
                for (o, s) in rgba
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(shade.rgba.as_chunks::<4>().0)
                {
                    o[3] = s[3];
                }
                let alpha = shade.rgba.as_chunks::<4>().0.iter().any(|p| p[3] < 128);
                let (w, h) = (shade.width as u32, shade.height as u32);
                let image = painted_image(w, h, rgba, true);
                // The painter's surface map (roughness in G, metal in B) as
                // the original's packed specular map: intensity, gloss, metal.
                let mut surface = painted.surface;
                for px in surface.as_chunks_mut::<4>().0 {
                    px[0] = 255;
                    px[1] = 255 - px[1];
                }
                let surface = painted_image(w, h, surface, false);
                let mut mat = ExaMaterial {
                    params: ExaParams {
                        specular: Vec4::new(1.0, 1.0, 1.0, 1.0),
                        flags: flags::LIT | flags::PACKED_SPECULAR,
                        ..default()
                    },
                    diffuse_map: Some(self.images.add(image)),
                    specular_map: Some(self.images.add(surface)),
                    environment: self.environment_map(),
                    ..default()
                };
                if alpha {
                    mat.alpha_mode = AlphaMode::Mask(0.5);
                    mat.params.flags |= flags::TWO_SIDED;
                }
                match painted.normal {
                    Some(normal) => {
                        let image = painted_image(w, h, normal, false);
                        mat.normal_map = Some(self.images.add(image));
                    }
                    None => {
                        mat.normal_map = self
                            .texture(&format!("{stem}n"), ColorSpace::Linear)
                            .map(|t| t.0);
                    }
                }
                self.materials.add(mat.with_map_flags())
            }
            None => self.apparel_material(stem),
        };
        self.cache.painted.insert(key, handle.clone());
        handle
    }

    /// The environment cube the original lights and reflects with
    /// (`genref01r` indoors): six faces with their mip chains, sRGB.
    pub fn environment_map(&mut self) -> Option<Handle<Image>> {
        if let Some(h) = &self.cache.environment {
            return h.clone();
        }
        let image = self
            .game
            .texture(ENVIRONMENT_MAP)
            .and_then(|b| Rfi::parse(b).ok())
            .and_then(|rfi| cube_image(&rfi));
        if image.is_none() {
            self.warn_once(format!(
                "environment map {ENVIRONMENT_MAP} could not be read"
            ));
        }
        let handle = image.map(|i| self.images.add(i));
        self.cache.environment = Some(handle.clone());
        handle
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

/// A layer's mask, material, colour and library textures, owned while painting.
type OwnedLayer = (
    Arc<Texels>,
    u8,
    u8,
    Vec<Option<Arc<Texels>>>,
    Vec<Option<Arc<Texels>>>,
);

/// An image with a box-filtered mip chain from RGBA8 pixels, sRGB or linear.
/// How a material blends, from its blend word: `0x4000` alpha-tests (when
/// a map gives alpha); otherwise the source and destination factors (bits
/// 8–11 and 0–3) pick opaque, premultiplied, additive or alpha blending.
fn blend_mode(word: u32, has_alpha: bool) -> AlphaMode {
    const ONE: u32 = 1;
    const SRC_ALPHA: u32 = 2;
    const ONE_MINUS_SRC_ALPHA: u32 = 3;
    if word & 0x4000 != 0 {
        return if has_alpha {
            AlphaMode::Mask(0.5)
        } else {
            AlphaMode::Opaque
        };
    }
    if word & 0x3F0F == 0x100 || word == 0 {
        return AlphaMode::Opaque;
    }
    match ((word >> 8) & 0xF, word & 0xF) {
        (ONE, ONE_MINUS_SRC_ALPHA) => AlphaMode::Premultiplied,
        (SRC_ALPHA, ONE) => AlphaMode::Add,
        (SRC_ALPHA, ONE_MINUS_SRC_ALPHA) => AlphaMode::Blend,
        _ => AlphaMode::Blend,
    }
}

/// The environment cube of indoor scenes.
const ENVIRONMENT_MAP: &str = "genref01r";

/// A cube map from an RGB8 `.rfi` holding six faces, each with its mips.
fn cube_image(rfi: &Rfi) -> Option<Image> {
    use bevy::render::render_resource::{
        Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
    };
    let (w, h) = (rfi.header.width, rfi.header.height);
    let levels: Vec<(u32, u32)> = (0..rfi.mip_count)
        .map(|m| ((w >> m).max(1), (h >> m).max(1)))
        .collect();
    let face: usize = levels.iter().map(|&(lw, lh)| (lw * lh * 3) as usize).sum();
    if rfi.format != exa_formats::rfi::PixelFormat::Rgb8 || rfi.data.len() < face * 6 {
        return None;
    }
    // RGB to RGBA, layer by layer with each layer's mips.
    let mut data = Vec::with_capacity(face * 6 * 4 / 3);
    for px in rfi.data[..face * 6].as_chunks::<3>().0 {
        data.extend_from_slice(&[px[0], px[1], px[2], 255]);
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = rfi.mip_count;
    image.data = Some(data);
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    Some(image)
}

fn painted_image(width: u32, height: u32, rgba: Vec<u8>, srgb: bool) -> Image {
    use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut data = rgba.clone();
    let (mut w, mut h, mut level) = (width as usize, height as usize, rgba);
    let mut levels = 1;
    while w > 1 && h > 1 && w.is_multiple_of(2) && h.is_multiple_of(2) {
        let (nw, nh) = (w / 2, h / 2);
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let at = |dx: usize, dy: usize| {
                        level[((2 * y + dy) * w + 2 * x + dx) * 4 + c] as u32
                    };
                    next[(y * nw + x) * 4 + c] =
                        ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                }
            }
        }
        data.extend_from_slice(&next);
        (w, h, level) = (nw, nh, next);
        levels += 1;
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        if srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = levels;
    image.data = Some(data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        anisotropy_clamp: 8,
        ..ImageSamplerDescriptor::linear()
    });
    image
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

/// Parallax scale per unit of the height layer's texel count.
const PARALLAX_UNIT: f32 = 3e-5;
/// Material extra flag (`0x4C`) enabling displacement shadows.
const DISPLACEMENT_SHADOWS: u32 = 0x10_0000;

/// A Bevy mesh holding only the vertices one material group uses.
fn group_mesh(mesh: &RfcMesh, indices: &[u32], skin: Option<&[VertexSkin]>) -> Option<Mesh> {
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
    if let Some(skin) = skin {
        let joints: Vec<[u16; 4]> = used.iter().map(|&i| skin[i].map(|(b, _)| b)).collect();
        let weights: Vec<[f32; 4]> = used.iter().map(|&i| skin[i].map(|(_, w)| w)).collect();
        m.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            bevy::mesh::VertexAttributeValues::Uint16x4(joints),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
    }
    Some(m)
}

/// Texture-name stem of a run-time material: the ten characters after the
/// `&`, lower case (`&doublet_ba1` → `doublet_ba`, `&gambsonaba1` →
/// `gambsonaba`), completed with `d`, `n` or `m<n>`.
pub fn apparel_texture_key(material: &str) -> Option<String> {
    let rest = material.strip_prefix('&')?;
    let stem: String = rest.chars().take(10).collect();
    (stem.len() == 10).then(|| stem.to_ascii_lowercase())
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
            apparel_texture_key("&gambsonaba1").as_deref(),
            Some("gambsonaba")
        );
        assert_eq!(apparel_texture_key("&short"), None);
        assert_eq!(apparel_texture_key("skinhuman"), None);
    }
}
