//! Chunk-tree models to Bevy meshes and materials, cached by name so each
//! model, material and texture is decoded and uploaded once.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_formats::material::{Material, TextureSlot};
use exa_formats::rfc::{Mesh as RfcMesh, Node, NodeBody, Scene};
use exa_formats::rfi::Rfi;
use exa_formats::rsq::bone_index;
use exa_rpk::{Archive, GameData, PackKind};

use crate::mesh_edit::material_split_flags;
use crate::mesh_smooth::smooth_mesh;
use crate::morph::{BuiltMorph, JobMorph, MorphCache, model_key};
use crate::paint::{
    Encoding, PaintLayer, Painted, Texels, layer_material, library, normal_library, paint,
};
use crate::skin::{Joint, Seams, VertexSkin, auto_skin};
use crate::texture::{self, ColorSpace, has_alpha, material_word, rfi_to_texture};
use crate::texture_cache::{TextureCache, TextureKey};
use exa_formats::material::flags as material_flags;
use exa_formats::objdb::Appearance;
use exa_render::material::{
    BloodProgram, ExaMaterial, ExaParams, SPATTER, flags, linear, linear_rgb,
};

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
    /// A voxel light: light the node adds into the GI volume only.
    pub voxel_light: Option<exa_formats::rfc::VoxelLight>,
    /// Size of the fire this node becomes (torches 0.2, braziers 0.41).
    pub fire: Option<f32>,
    /// Size of the level portal marker this node is (`uwportal01`'s
    /// `_PortalPrt`: 100), which the game replaces with the portal's sparks
    /// and light.
    pub portal: Option<f32>,
    /// Bounding sphere in the node's space (metres).
    pub bounds: (Vec3, f32),
    /// Half extents of the node's bounding box about the sphere's centre
    /// (metres): a fist's length runs across the hand's box.
    pub half_extents: Vec3,
    /// The mesh's GI sample points in the node's space (metres), which mark
    /// the voxels it fills.
    pub gi_points: Arc<Vec<Vec3>>,
    /// The node's rigid body, when it carries one that decodes and the
    /// node is not marked as level geometry
    /// ([`exa_formats::rfc::Node::is_static`]).
    pub rigid: Option<NodeRigidBody>,
    /// The node's keyframe tracks (curves of its translation, played when
    /// they are started).
    pub tracks: Vec<exa_formats::key_track::KeyTrack>,
    /// The mesh's shadow-bias level from its file (0..=7,
    /// [`exa_formats::rfc::Mesh::shadow_bias_level`]); 0 for other nodes
    /// and for the inflated variant.
    pub shadow_bias: u32,
}

/// A model node's rigid body and what its mobility is divided by when the
/// body is registered: the physical divisor of the material of the node's
/// first mesh part ([`Material::mobility_divisor`], 1 without a material).
#[derive(Debug, Clone)]
pub struct NodeRigidBody {
    pub body: Arc<exa_formats::rigid_body::RigidBody>,
    pub mobility_divisor: f32,
}

impl NodeRigidBody {
    /// The body's mobility as the original registers it: the stored value
    /// over the divisor.
    pub fn mobility(&self) -> f32 {
        self.body.mobility / self.mobility_divisor
    }
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
    /// The texture manager: records keyed by name and colour space, with
    /// their sampler words and reference counts.
    pub textures: TextureCache,
    materials: HashMap<String, Handle<ExaMaterial>>,
    /// Textures decoded on the CPU, for painting.
    texels: HashMap<String, Option<Arc<Texels>>>,
    /// Painted apparel materials by texture stem and appearance.
    painted: HashMap<(String, Appearance), Handle<ExaMaterial>>,
    /// The environment cube, once looked for.
    environment: Option<Option<Handle<Image>>>,
    /// Body model stems (lower case) of the races that bleed, once read.
    bleeding_stems: Option<HashSet<String>>,
    /// Mobility divisors by material cache key ([`ModelLoader::mobility_divisor`]).
    divisors: HashMap<String, f32>,
    /// Morph bodies and the shares of the morphs models take
    /// ([`crate::morph`]).
    pub(crate) morph: MorphCache,
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
        let key = model_key(pack, entry);
        if let Some(cached) = self.cache.models.get(&key) {
            return cached.clone();
        }
        let model = self.load(pack, entry).map(Arc::new);
        self.cache.models.insert(key, model.clone());
        model
    }

    /// The model of `entry` with every mesh smoothed as the original's
    /// mesh-smoothing pass does it ([`crate::mesh_smooth::smooth_mesh`]):
    /// what its character-creation scene does to the hair it shows, and
    /// its `msm` debug command to every mesh of a scene. Cached apart from
    /// the plain model.
    pub fn smoothed_model(&mut self, pack: PackKind, entry: &str) -> Option<Arc<Model>> {
        let key = format!("{}#smooth", model_key(pack, entry));
        if let Some(cached) = self.cache.models.get(&key) {
            return cached.clone();
        }
        let model = self.model_job(pack, entry).map(|mut job| {
            job.smooth = Some(self.split_flags(&job.scene));
            let built = job.build(bevy::tasks::ComputeTaskPool::try_get());
            Arc::new(self.assemble(built))
        });
        self.cache.models.insert(key, model.clone());
        model
    }

    /// The split flags of every mesh node's groups, from their materials
    /// (the file's own, else the libraries'; none for run-time names).
    fn split_flags(&self, scene: &Scene) -> Vec<Vec<u32>> {
        scene
            .nodes
            .iter()
            .map(|node| match &node.body {
                NodeBody::Mesh(mesh) => mesh
                    .groups
                    .iter()
                    .map(|g| {
                        scene
                            .material(&g.material)
                            .cloned()
                            .or_else(|| library_material(self.game, &g.material))
                            .map_or(0, |m| material_split_flags(&m))
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// Loads the materials of a model, and so their textures, until
    /// `deadline`, without building the model; `true` once they are all
    /// loaded (or the model cannot be). A model with many textures takes a
    /// few hundred milliseconds to load: warming its materials over several
    /// frames first lets the model itself load within one.
    pub fn warm_materials(
        &mut self,
        pack: PackKind,
        entry: &str,
        deadline: std::time::Instant,
    ) -> bool {
        let key = format!("{}/{}", pack.file_stem(), entry.to_ascii_lowercase());
        if self.cache.models.contains_key(&key) {
            return true;
        }
        let Some(scene) = self
            .game
            .pack(pack)
            .and_then(|p| p.get(entry))
            .and_then(|b| Scene::parse(b).ok())
        else {
            return true;
        };
        // The materials `parts` gives the model's meshes; at least one new
        // one per call, so warming always gets somewhere.
        let mut loaded = false;
        for node in &scene.nodes {
            let NodeBody::Mesh(mesh) = &node.body else {
                continue;
            };
            for group in &mesh.groups {
                if group.index_count == 0 || group.material.eq_ignore_ascii_case("_null") {
                    continue;
                }
                let key = Self::material_key(entry, &scene, &group.material);
                if self.cache.materials.contains_key(&key) {
                    continue;
                }
                if loaded && std::time::Instant::now() >= deadline {
                    return false;
                }
                self.material(entry, &scene, &group.material);
                loaded = true;
            }
        }
        true
    }

    fn load(&mut self, pack: PackKind, entry: &str) -> Option<Model> {
        let job = self.model_job(pack, entry)?;
        let built = job.build(bevy::tasks::ComputeTaskPool::try_get());
        Some(self.assemble(built))
    }

    /// The model of `entry` in `pack` when it has been loaded (`Some(None)`
    /// when it could not be).
    pub fn loaded_model(&self, pack: PackKind, entry: &str) -> Option<Option<Arc<Model>>> {
        let key = format!("{}/{}", pack.file_stem(), entry.to_ascii_lowercase());
        self.cache.models.get(&key).cloned()
    }

    /// What building a model takes, owned so that [`ModelJob::build`] can run
    /// on any thread: the parsed file and, for apparel, the body's weights
    /// it takes its own from. `None` when the file is missing or does not
    /// parse (warned once). [`ModelLoader::finish_model`] then adds the
    /// built meshes and the materials, as [`ModelLoader::model`] does at
    /// once.
    pub fn model_job(&mut self, pack: PackKind, entry: &str) -> Option<ModelJob> {
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
        Some(ModelJob {
            pack,
            entry: entry.to_string(),
            scene,
            morph: None,
            smooth: None,
        })
    }

    /// The model a [`ModelJob`] built, its meshes and materials added and
    /// cached as [`ModelLoader::model`] caches it.
    pub fn finish_model(&mut self, built: BuiltModel) -> Option<Arc<Model>> {
        let key = match &built.morph {
            Some(m) => m.key.clone(),
            None => model_key(built.pack, &built.entry),
        };
        let model = Some(Arc::new(self.assemble(built)));
        self.cache.models.insert(key, model.clone());
        model
    }

    /// Adds a built model's meshes and materials, and remembers the shares
    /// of the morphs a morphed variant used.
    fn assemble(&mut self, built: BuiltModel) -> Model {
        let BuiltModel {
            pack: _,
            entry,
            scene,
            nodes: built_nodes,
            morph,
        } = built;
        if let Some(m) = morph {
            self.cache.morph.parts.insert(m.parts_key, m.parts);
        }
        let mut nodes = Vec::with_capacity(scene.nodes.len());
        for (node, built) in scene.nodes.iter().zip(built_nodes) {
            let skin_bone = built.skin.as_ref().map(|(bone, _)| *bone);
            // A node marked as level geometry never registers its body.
            let rigid =
                node.rigid()
                    .filter(|_| !node.is_static())
                    .map(|body| NodeRigidBody {
                        body: Arc::new(body.clone()),
                        mobility_divisor: match &node.body {
                            NodeBody::Mesh(mesh) => mesh.groups.first().map_or(1.0, |g| {
                                self.mobility_divisor(&entry, &scene, &g.material)
                            }),
                            _ => 1.0,
                        },
                    });
            let (parts, part_materials) = match &node.body {
                NodeBody::Mesh(mesh) => built
                    .meshes
                    .into_iter()
                    .map(|(group, bevy_mesh)| {
                        let name = &mesh.groups[group].material;
                        let material = self.material(&entry, &scene, name);
                        ((self.meshes.add(bevy_mesh), material), name.clone())
                    })
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
                half_extents: Vec3::from(node.bounds_half_extents),
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
                portal: match &node.body {
                    NodeBody::Emitter { kind, size } if *kind == exa_formats::rfc::PORTAL => {
                        Some(*size)
                    }
                    _ => None,
                },
                light: match &node.body {
                    NodeBody::Light(l) => Some(*l),
                    _ => None,
                },
                voxel_light: match &node.body {
                    NodeBody::VoxelLight(l) => Some(*l),
                    _ => None,
                },
                rigid,
                tracks: node.tracks().cloned().collect(),
                shadow_bias: match &node.body {
                    // The inflated variant (both sides × (1 + 0.005·v)) is
                    // not reproduced: such a mesh keeps level 0.
                    NodeBody::Mesh(mesh) if !mesh.shadow_bias_inflated() => {
                        mesh.shadow_bias_level()
                    }
                    _ => 0,
                },
            });
        }
        Model { nodes }
    }

    /// The cache key of a model's material: its own, or the library's.
    fn material_key(entry: &str, scene: &Scene, name: &str) -> String {
        if scene.material(name).is_some() {
            format!(
                "{}#{}",
                entry.to_ascii_lowercase(),
                name.to_ascii_lowercase()
            )
        } else {
            format!("lib#{}", name.to_ascii_lowercase())
        }
    }

    /// What a rigid body's stored mobility is divided by when its node's
    /// first mesh part has the material `name`: the material's physical
    /// divisor (its own record, else the libraries'), 1 when it has none.
    fn mobility_divisor(&mut self, entry: &str, scene: &Scene, name: &str) -> f32 {
        let key = Self::material_key(entry, scene, name);
        if let Some(&d) = self.cache.divisors.get(&key) {
            return d;
        }
        let d = scene
            .material(name)
            .cloned()
            .or_else(|| library_material(self.game, name))
            .map_or(1.0, |m| m.mobility_divisor());
        self.cache.divisors.insert(key, d);
        d
    }

    fn material(&mut self, entry: &str, scene: &Scene, name: &str) -> Handle<ExaMaterial> {
        let local = scene.material(name).cloned();
        let key = Self::material_key(entry, scene, name);
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
            // The body's skin is drawn as the original's built-in skin.
            Some(_) if crate::human_skin::is_race_skin(entry, name) => {
                crate::human_skin::skin_material(self, entry)
            }
            Some(mut m) => {
                // A bleeding race's skin takes the blood dirt mode, as the
                // original's race loader sets it.
                if is_body_model(entry) && takes_race_blood(name) && self.race_bleeds(entry) {
                    m.extra |= DIRT_MODE_BLOOD;
                }
                self.exa_material(&m)
            }
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
                emissive: emissive(m),
                flags: shading,
                ..default()
            },
            ..default()
        };
        // The record's texture load flags, with the mode bits the original
        // adds per slot: normal, gloss and height maps (and the specular map
        // of a material whose kind has bit 0) are data, and a diffuse with a
        // secondary colour skips the LOD bias.
        let settings = *self.cache.textures.settings();
        let base_flags = material_word(m.texture_flags, &settings);
        let data_flags = base_flags | texture::flags::DATA | texture::flags::NO_LOD_BIAS;
        let bias_skipped = base_flags | texture::flags::NO_LOD_BIAS;
        let mut cut_out = false;
        if let Some(t) = m.texture(TextureSlot::Diffuse) {
            let flags = if m.has_secondary_colour() {
                bias_skipped
            } else {
                base_flags
            };
            if let Some((img, alpha)) = self.texture_with_flags(t, ColorSpace::Srgb, flags) {
                mat.diffuse_map = Some(img);
                cut_out = alpha;
            }
        }
        if let Some(t) = m.texture(TextureSlot::Normal) {
            mat.normal_map = self
                .texture_with_flags(t, ColorSpace::Linear, data_flags)
                .map(|t| t.0);
        }
        if let Some(t) = m.texture(TextureSlot::Specular) {
            let space = if packed {
                ColorSpace::Linear
            } else {
                ColorSpace::Srgb
            };
            let flags = if m.kind & 1 != 0 {
                data_flags
            } else {
                base_flags
            };
            mat.specular_map = self.texture_with_flags(t, space, flags).map(|t| t.0);
        }
        if let Some(t) = m.texture(TextureSlot::Emissive) {
            mat.emissive_map = self
                .texture_with_flags(t, ColorSpace::Srgb, base_flags)
                .map(|t| t.0);
        }
        if let Some(t) = m.texture(TextureSlot::Opacity) {
            mat.opacity_map = self
                .texture_with_flags(t, ColorSpace::Linear, base_flags)
                .map(|t| t.0);
        }
        // Dirt maps only act through vertex colours, which level meshes lack.
        if let Some(t) = m.texture(TextureSlot::Gloss) {
            mat.gloss_map = self
                .texture_with_flags(t, ColorSpace::Linear, data_flags)
                .map(|t| t.0);
        }
        // Parallax: the height layer's depth as a fixed number of texels of
        // the height map; displacement shadows when the record asks.
        if let Some(layer) = m.layer(TextureSlot::Height)
            && layer.amount > 0.0
            && mat.normal_map.is_some()
            && let Some((img, _)) =
                self.texture_with_flags(&layer.texture, ColorSpace::Linear, data_flags)
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
        if mat.alpha_mode == AlphaMode::Add {
            mat.params.flags |= flags::ADDITIVE;
        }
        // Faint surfaces (glass, decals) and those the record marks cast no
        // shadows.
        mat.no_shadows = m.opacity <= 0.8 || m.extra & NO_SHADOWS != 0;
        mat.cut_shadow = cuts_shadow(m, mat.opacity_map.is_some());
        if mat.alpha_mode == AlphaMode::Premultiplied {
            let a = mat.params.diffuse.w;
            mat.params.diffuse = (mat.params.diffuse.truncate() * a).extend(a);
        }
        // The fall-off's floor comes off the opacity (after the
        // premultiplication, which takes the opacity as stored).
        if let Some((f, floor)) = opacity_falloff(m) {
            mat.params.opacity_falloff = f;
            mat.params.diffuse.w -= floor;
            mat.params.flags |= flags::OPACITY_FALLOFF;
        }
        mat.environment = self.environment_map();
        // Dirt mode 4 (blood) needs a diffuse map and a program with dirt.
        if m.extra & DIRT_MODE == DIRT_MODE_BLOOD
            && mat.diffuse_map.is_some()
            && let Some(program) = BloodProgram::of_type(m.kind)
        {
            let spatter = self.spatter();
            mat = mat.with_blood(program, spatter);
        }
        mat.with_map_flags()
    }

    /// The material of a run-time (`&`) apparel texture set without its
    /// painting, as the original's `_procmat1` built-in replaces it: blood
    /// shows on it as on cloth.
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
        if mat.diffuse_map.is_some() {
            let spatter = self.spatter();
            mat = mat.with_blood(BloodProgram::Cloth, spatter);
        }
        self.materials.add(mat.with_map_flags())
    }

    /// The speckle texture blood shows through (`spatter01`, read as data).
    pub fn spatter(&mut self) -> Option<Handle<Image>> {
        self.texture(SPATTER, ColorSpace::Linear).map(|t| t.0)
    }

    /// Whether a race whose body model is `entry` bleeds (`races.rdb`).
    fn race_bleeds(&mut self, entry: &str) -> bool {
        let game = self.game;
        let stems = self.cache.bleeding_stems.get_or_insert_with(|| {
            game.pack(PackKind::Resource)
                .and_then(|p| p.get("races.rdb"))
                .and_then(|b| exa_formats::races::parse(b).ok())
                .unwrap_or_default()
                .iter()
                .filter(|r| r.bleeds())
                .map(|r| r.stem.to_ascii_lowercase())
                .collect()
        });
        let lower = entry.to_ascii_lowercase();
        lower
            .strip_suffix("base.rfc")
            .is_some_and(|stem| stems.contains(stem))
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
    /// missing. Paints on the calling thread; [`ModelLoader::paint_job`]
    /// lets the painting run elsewhere.
    pub fn painted_apparel(&mut self, stem: &str, appearance: &Appearance) -> Handle<ExaMaterial> {
        match self.paint_job(stem, appearance) {
            Paint::Ready(handle) => handle,
            Paint::Job(job) => {
                let painted = job.paint();
                self.painted_material(painted)
            }
        }
    }

    /// Decodes the textures painting `stem` with `appearance` reads, one at
    /// a time until `deadline` (at least one per call); `true` once they are
    /// all decoded and [`ModelLoader::paint_job`] only reads the cache. A
    /// texture takes tens of milliseconds to decode, a texture set's dozen
    /// of them too long for one frame.
    pub fn warm_paint(
        &mut self,
        stem: &str,
        appearance: &Appearance,
        deadline: std::time::Instant,
    ) -> bool {
        if self
            .cache
            .painted
            .contains_key(&(stem.to_ascii_lowercase(), *appearance))
        {
            return true;
        }
        let mut names = vec![format!("{stem}d")];
        for layer in appearance.used_layers() {
            let mask = format!("{stem}m{}", layer.mask);
            if self.game.texture(&mask).is_none() {
                continue;
            }
            names.push(mask);
            let material = layer_material(layer);
            names.extend(library(material).iter().map(|n| n.to_string()));
            names.extend(normal_library(material).iter().map(|n| n.to_string()));
        }
        names.push(format!("{stem}n"));
        let mut decoded = false;
        for (i, name) in names.iter().enumerate() {
            let cached = self.cache.texels.get(&name.to_ascii_lowercase()).cloned();
            let texels = match cached {
                Some(texels) => texels,
                None => {
                    if decoded && std::time::Instant::now() >= deadline {
                        return false;
                    }
                    decoded = true;
                    self.texels(name)
                }
            };
            // Without a shade map nothing is painted.
            if i == 0 && texels.is_none() {
                return true;
            }
        }
        true
    }

    /// The painted material of `stem` with `appearance` when it is ready
    /// (painted before, or the unpainted material standing in for a missing
    /// shade map); otherwise what painting it takes, gathered from the
    /// decoded-texture cache.
    pub fn paint_job(&mut self, stem: &str, appearance: &Appearance) -> Paint {
        let key = (stem.to_ascii_lowercase(), *appearance);
        if let Some(h) = self.cache.painted.get(&key) {
            return Paint::Ready(h.clone());
        }
        let Some(shade) = self.texels(&format!("{stem}d")) else {
            let handle = self.apparel_material(stem);
            self.cache.painted.insert(key, handle.clone());
            return Paint::Ready(handle);
        };
        let mut layers: Vec<OwnedLayer> = Vec::new();
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
            layers.push((mask, material, layer.colour, lib, normals));
        }
        let item_normal = self.texels(&format!("{stem}n"));
        Paint::Job(PaintJob {
            key,
            shade,
            item_normal,
            layers,
        })
    }

    /// The material of a painted texture set, cached for
    /// [`ModelLoader::painted_apparel`].
    pub fn painted_material(&mut self, painted: PaintedSet) -> Handle<ExaMaterial> {
        let PaintedSet {
            key,
            colour,
            surface,
            normal,
            alpha,
        } = painted;
        let mut mat = ExaMaterial {
            params: ExaParams {
                specular: Vec4::new(1.0, 1.0, 1.0, 1.0),
                flags: flags::LIT | flags::PACKED_SPECULAR,
                ..default()
            },
            diffuse_map: Some(self.images.add(colour)),
            specular_map: Some(self.images.add(surface)),
            environment: self.environment_map(),
            ..default()
        };
        if alpha {
            mat.alpha_mode = AlphaMode::Mask(0.5);
            mat.params.flags |= flags::TWO_SIDED;
        }
        mat.normal_map = match normal {
            Some(image) => Some(self.images.add(image)),
            None => self
                .texture(&format!("{}n", key.0), ColorSpace::Linear)
                .map(|t| t.0),
        };
        // The original's painted apparel (`_procmat1`) and factory items
        // (`_procmatr`) show blood as cloth.
        let spatter = self.spatter();
        let handle = self
            .materials
            .add(mat.with_blood(BloodProgram::Cloth, spatter));
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

    /// Image handle and whether it has alpha, with the default texture word
    /// (a material texture at the retail settings).
    pub fn texture(&mut self, name: &str, space: ColorSpace) -> CachedTexture {
        self.texture_with_flags(name, space, crate::texture::default_flags())
    }

    /// Image handle and whether it has alpha, with the creation/sampler
    /// word a material asks for. The image is looked up by its stored name
    /// (cut at the last dot, folded) and decoded once; a live record keeps
    /// the word it was created with, as the original's manager does.
    ///
    /// An image that is shipped but cannot be read uploads the original's
    /// 1x1 grey placeholder, which the record keeps. One the game does not
    /// ship is no map: the original would draw the placeholder too, but
    /// Exarust has no registry of the images it builds at run time
    /// (painted apparel, skins), so a name it cannot find may be one the
    /// original makes.
    pub fn texture_with_flags(
        &mut self,
        name: &str,
        space: ColorSpace,
        flags: u32,
    ) -> CachedTexture {
        let key = TextureKey::new(name, space);
        let created = self.cache.textures.acquire(key, flags).created;
        if created {
            let settings = *self.cache.textures.settings();
            let Some(bytes) = self.game.texture(key.name.as_str()) else {
                self.warn_once(format!("texture {name:?} not found"));
                return None;
            };
            let result = Rfi::parse(bytes)
                .and_then(|rfi| {
                    let alpha = has_alpha(&rfi);
                    rfi_to_texture(&rfi, space, self.bc_supported, flags, &settings)
                        .map(|img| (img, alpha))
                })
                .map_err(|e| format!("texture {name:?}: {e}"));
            match result {
                Ok((img, alpha)) => {
                    let handle = self.images.add(img);
                    self.cache.textures.finish(&key, handle, alpha);
                }
                Err(msg) => {
                    self.warn_once(msg);
                    let image = crate::texture::placeholder_image(space, flags, &settings);
                    let handle = self.images.add(image);
                    self.cache.textures.finish(&key, handle, false);
                }
            }
        }
        self.cache
            .textures
            .get(&key)
            .and_then(|record| record.image.clone().map(|image| (image, record.alpha)))
    }
}

/// A model to build: its parsed file, and for a character's variant what
/// morphing it takes ([`ModelLoader::morphed_job`]).
pub struct ModelJob {
    pack: PackKind,
    entry: String,
    scene: Scene,
    pub(crate) morph: Option<JobMorph>,
    /// Smooth every mesh node first, with these split flags per node's
    /// groups ([`ModelLoader::smoothed_model`]).
    smooth: Option<Vec<Vec<u32>>>,
}

/// A model's meshes, built and not yet added as assets
/// ([`ModelLoader::finish_model`]).
pub struct BuiltModel {
    pack: PackKind,
    entry: String,
    scene: Scene,
    nodes: Vec<BuiltNode>,
    morph: Option<BuiltMorph>,
}

/// A node's skin weights (with the bone whose space they are in) and its
/// drawn mesh groups' meshes, by group index.
struct BuiltNode {
    skin: Option<(u16, Vec<VertexSkin>)>,
    meshes: Vec<(usize, Mesh)>,
}

impl ModelJob {
    /// Works out the skin weights and builds the meshes, the groups of a
    /// node on every thread of `pool` when given (one by one, a large
    /// model's meshes, their tangents most of all, took hundreds of
    /// milliseconds).
    pub fn build(self, pool: Option<&bevy::tasks::ComputeTaskPool>) -> BuiltModel {
        let ModelJob {
            pack,
            entry,
            mut scene,
            morph,
            smooth,
        } = self;
        if let Some(flags) = &smooth {
            for (node, flags) in scene.nodes.iter_mut().zip(flags) {
                smooth_node(node, flags);
            }
        }
        let body_model = pack == PackKind::Resource && is_body_model(&entry);
        let skinned = pack == PackKind::Characters || body_model;
        let skins_of = |scene: &Scene| -> Vec<Option<(u16, Vec<VertexSkin>)>> {
            scene
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| match &node.body {
                    NodeBody::Mesh(mesh) if skinned => node_skin(scene, index, mesh, pack),
                    _ => None,
                })
                .collect()
        };
        // A garment takes its weights from the unmorphed body before it is
        // morphed; the body's parts are skinned on their morphed shape and
        // joints.
        let unmorphed = (morph.is_none() || !body_model).then(|| skins_of(&scene));
        let morph = morph.map(|m| {
            let parts = m.parts(&scene, body_model);
            m.apply(&mut scene, &parts);
            m.built(parts)
        });
        let skins = unmorphed.unwrap_or_else(|| skins_of(&scene));
        let nodes = scene
            .nodes
            .iter()
            .zip(skins)
            .map(|(node, skin)| {
                let NodeBody::Mesh(mesh) = &node.body else {
                    return BuiltNode {
                        skin: None,
                        meshes: Vec::new(),
                    };
                };
                let meshes = group_meshes(mesh, skin.as_ref().map(|(_, s)| s.as_slice()), pool);
                BuiltNode { skin, meshes }
            })
            .collect();
        BuiltModel {
            pack,
            entry,
            scene,
            nodes,
            morph,
        }
    }
}

/// Smooths a mesh node in place and refits its bounds; a mesh that cannot
/// be smoothed is left as it is, with the original's warnings.
fn smooth_node(node: &mut Node, flags: &[u32]) {
    let NodeBody::Mesh(mesh) = &mut node.body else {
        return;
    };
    match smooth_mesh(mesh, flags) {
        Ok(Some(smoothed)) => {
            if smoothed.degenerate_faces > 0 {
                warn!(
                    "Meshsmooth: {} degenerate faces found in {}",
                    smoothed.degenerate_faces, node.name
                );
            }
            if let Some((centre, half, radius)) = smoothed.bounds {
                node.bounds_centre = centre;
                node.bounds_half_extents = half;
                node.bounds_radius = radius;
            }
            **mesh = smoothed.mesh;
        }
        Ok(None) => warn!("MeshSmooth: {} has too many faces to subdivide", node.name),
        Err(e) => warn!("MeshSmooth: {}: {e}", node.name),
    }
}

/// Skin weights of a node's mesh: worked out from its seams for body parts
/// and garments alike (nodes named after a bone), with the parent and the
/// bone-named children as joints. A `Resource` model of a race body is a
/// body part; a `Characters` model is apparel, whose seams are the marked
/// ones.
fn node_skin(
    scene: &Scene,
    index: usize,
    mesh: &RfcMesh,
    pack: PackKind,
) -> Option<(u16, Vec<VertexSkin>)> {
    let node = &scene.nodes[index];
    // Nodes the original leaves out of the skin (its flag bit 5).
    if node.flags & 0x20 != 0 {
        return None;
    }
    let bone = bone_index(&node.name)? as u16;
    let bone_of = |i: usize| bone_index(&scene.nodes[i].name).map(|b| b as u16);
    let mut joints = Vec::new();
    if let Some(parent) = node.parent.and_then(|p| p.checked_sub(1))
        && let NodeBody::Mesh(_) = &scene.nodes[parent].body
        && let Some(b) = bone_of(parent)
    {
        joints.push(Joint {
            bone: b,
            pivot: Vec3::ZERO,
        });
    }
    for (i, child) in scene.nodes.iter().enumerate() {
        if child.parent == Some(index + 1)
            && let NodeBody::Mesh(_) = &child.body
            && let Some(b) = bone_of(i)
        {
            joints.push(Joint {
                bone: b,
                pivot: Vec3::from(child.transform[3]),
            });
        }
    }
    if mesh.skin.is_none() && (joints.is_empty() || mesh.triangle_flags.is_none()) {
        return None;
    }
    let seams = if pack == PackKind::Characters {
        Seams::Apparel
    } else {
        Seams::Body
    };
    Some((bone, auto_skin(mesh, bone, &joints, seams)))
}

/// The meshes of a mesh's drawn groups (`_null` marks geometry the
/// original never draws), by group index, built on `pool` when given.
fn group_meshes(
    mesh: &RfcMesh,
    skin: Option<&[VertexSkin]>,
    pool: Option<&bevy::tasks::ComputeTaskPool>,
) -> Vec<(usize, Mesh)> {
    let groups: Vec<usize> = (0..mesh.groups.len())
        .filter(|&g| !mesh.groups[g].material.eq_ignore_ascii_case("_null"))
        .collect();
    let build = |g: usize| {
        let group = &mesh.groups[g];
        let start = group.first_index as usize;
        mesh.indices
            .get(start..start + group.index_count as usize)
            .and_then(|indices| group_mesh(mesh, indices, skin))
            .map(|m| (g, m))
    };
    let built: Vec<Option<(usize, Mesh)>> = match pool {
        Some(pool) if groups.len() > 1 => pool.scope(|scope| {
            for &g in &groups {
                scope.spawn(async move { build(g) });
            }
        }),
        _ => groups.iter().map(|&g| build(g)).collect(),
    };
    built.into_iter().flatten().collect()
}

/// A layer's mask, material, colour and library textures, owned while painting.
type OwnedLayer = (
    Arc<Texels>,
    u8,
    u8,
    Vec<Option<Arc<Texels>>>,
    Vec<Option<Arc<Texels>>>,
);

/// A record's emissive term as the shader takes it (the original's MatEmi):
/// the linear colour, doubled for shaded materials (lit or specular bits)
/// with an emissive layer; alpha the opacity for the others, the colour's
/// own alpha for shaded ones.
fn emissive(m: &Material) -> Vec4 {
    let shaded = m.flags & SHADED != 0;
    let mut rgb = linear_rgb([m.emissive[0], m.emissive[1], m.emissive[2]]);
    if shaded && m.flags & EMISSIVE_LAYER != 0 {
        rgb *= 2.0;
    }
    let alpha = if shaded {
        m.emissive[3]
    } else {
        m.opacity.clamp(0.0, 1.0)
    };
    rgb.extend(alpha)
}

/// The opacity fall-off of a record whose amount (`0x64`) is above 0.01 in
/// magnitude, whatever its blending, and the floor it takes off the
/// opacity: with f = 2^(−1.8·exponent), (f + 1)·f.
fn opacity_falloff(m: &Material) -> Option<(Vec4, f32)> {
    (m.falloff_amount.abs() > FALLOFF_MIN).then(|| {
        let f = (-1.8 * m.falloff_exponent).exp2();
        let params = ExaParams::falloff(
            m.falloff_amount,
            m.falloff_exponent,
            m.extra & FALLOFF_INVERTED != 0,
        );
        (params, (f + 1.0) * f)
    })
}

/// Whether the original casts a material through its alpha-tested caster,
/// whatever the surface's blending: an alpha-tested blend word (`0x4000`),
/// or an opacity above 0.75 with the opacity flag, which the record sets or
/// a loaded opacity map adds (`cobweb01`: blended, opacity 1).
/// `opacity_map` is whether the opacity layer's texture loaded.
fn cuts_shadow(m: &Material, opacity_map: bool) -> bool {
    let opacity_flag = m.flags & OPACITY_FLAG != 0 || opacity_map;
    m.class & ALPHA_TESTED != 0 || opacity_flag && m.opacity > CUT_SHADOW_OPACITY
}

/// A painted apparel material, or what painting it takes.
pub enum Paint {
    Ready(Handle<ExaMaterial>),
    Job(PaintJob),
}

/// Everything painting an apparel texture set with an appearance reads,
/// shared from the decoded-texture cache: the painting (per-texel work, a
/// large share of dressing a crowd) can run on any thread.
pub struct PaintJob {
    key: (String, Appearance),
    shade: Arc<Texels>,
    item_normal: Option<Arc<Texels>>,
    layers: Vec<OwnedLayer>,
}

/// A painted texture set's images, for [`ModelLoader::painted_material`].
pub struct PaintedSet {
    key: (String, Appearance),
    colour: Image,
    surface: Image,
    normal: Option<Image>,
    /// The shade map cuts the item out.
    alpha: bool,
}

impl PaintedSet {
    /// Bytes of image data the set uploads to the GPU.
    pub fn bytes(&self) -> usize {
        [
            Some(&self.colour),
            Some(&self.surface),
            self.normal.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter_map(|image| image.data.as_ref())
        .map(Vec::len)
        .sum()
    }
}

impl PaintJob {
    /// Paints the texture set: colour, surface and normal images with their
    /// mip chains.
    pub fn paint(self) -> PaintedSet {
        let layers: Vec<PaintLayer<'_>> = self
            .layers
            .iter()
            .map(|(mask, material, colour, lib, normals)| PaintLayer {
                mask,
                material: *material,
                colour: *colour,
                library: lib.iter().map(|t| t.as_deref()).collect(),
                normals: normals.iter().map(|t| t.as_deref()).collect(),
            })
            .collect();
        let shade = &self.shade;
        let painted = paint(shade, self.item_normal.as_deref(), &layers, &self.key.1);
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
        // The painter's surface map (roughness in G, metal in B) as the
        // original's packed specular map: intensity, gloss, metal.
        let mut surface = painted.surface;
        for px in surface.as_chunks_mut::<4>().0 {
            px[0] = 255;
            px[1] = 255 - px[1];
        }
        PaintedSet {
            colour: painted_image(w, h, rgba, Painted::COLOUR),
            surface: painted_image(w, h, surface, Painted::SURFACE),
            normal: painted
                .normal
                .map(|n| painted_image(w, h, n, Painted::NORMAL)),
            alpha,
            key: self.key,
        }
    }
}

/// How a material blends, from its blend word: `0x4000` alpha-tests (when
/// a map gives alpha); otherwise the source and destination factors (bits
/// 8–11 and 0–3) pick opaque, premultiplied, additive or alpha blending.
fn blend_mode(word: u32, has_alpha: bool) -> AlphaMode {
    const ONE: u32 = 1;
    const SRC_ALPHA: u32 = 2;
    const ONE_MINUS_SRC_ALPHA: u32 = 3;
    if word & ALPHA_TESTED != 0 {
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

/// An image of a painted map with a box-filtered mip chain. An sRGB map keeps
/// its texels as painted and averages its colour in linear light, as the GPU
/// builds the mips of the original's sRGB paint targets; alpha and linear maps
/// are averaged as stored.
fn painted_image(width: u32, height: u32, rgba: Vec<u8>, encoding: Encoding) -> Image {
    use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let srgb = SrgbTables::new();
    let mut data = rgba.clone();
    let (mut w, mut h, mut level) = (width as usize, height as usize, rgba);
    let mut levels = 1;
    while w > 1 && h > 1 && w.is_multiple_of(2) && h.is_multiple_of(2) {
        let (nw, nh) = (w / 2, h / 2);
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let at = |dx: usize, dy: usize| level[((2 * y + dy) * w + 2 * x + dx) * 4 + c];
                    let texels = [at(0, 0), at(1, 0), at(0, 1), at(1, 1)];
                    next[(y * nw + x) * 4 + c] = if c < 3 && encoding == Encoding::Srgb {
                        srgb.mean(texels)
                    } else {
                        ((texels.iter().map(|&t| t as u32).sum::<u32>() + 2) / 4) as u8
                    };
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
        match encoding {
            Encoding::Srgb => TextureFormat::Rgba8UnormSrgb,
            Encoding::Linear => TextureFormat::Rgba8Unorm,
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

/// sRGB texel values in linear light, and the linear values at which encoding
/// rounds up to the next texel value, for averaging sRGB texels in linear
/// light.
struct SrgbTables {
    decode: [f32; 256],
    /// `round_up[k]`: the linear value from which `k + 1` encodes it.
    round_up: [f32; 255],
}

impl SrgbTables {
    fn new() -> Self {
        Self {
            decode: std::array::from_fn(|i| linear(i as f32 / 255.0)),
            round_up: std::array::from_fn(|k| linear((k as f32 + 0.5) / 255.0)),
        }
    }

    /// The sRGB texel value encoding the linear mean of four texels.
    fn mean(&self, texels: [u8; 4]) -> u8 {
        let mean = texels.iter().map(|&t| self.decode[t as usize]).sum::<f32>() / 4.0;
        self.round_up.partition_point(|&r| r <= mean) as u8
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

/// Parallax scale per unit of the height layer's texel count.
const PARALLAX_UNIT: f32 = 3e-5;
/// Material extra flag (`0x4C`): casts no shadows.
const NO_SHADOWS: u32 = 0x4;
/// A material with the opacity flag and an opacity above this casts a cut
/// shadow, whatever its blending.
const CUT_SHADOW_OPACITY: f32 = 0.75;
/// Material flag (`0x48`) of the opacity layer, also set when its texture
/// loads. Not every record that sets it has the layer (`harhps1`), nor the
/// reverse.
const OPACITY_FLAG: u32 = 0x40_0000;
/// Blend-word bit of alpha-tested materials.
const ALPHA_TESTED: u32 = 0x4000;
/// Material extra flag (`0x4C`) enabling displacement shadows.
const DISPLACEMENT_SHADOWS: u32 = 0x10_0000;
/// Material extra flag (`0x4C`): the opacity fall-off fades toward the
/// silhouette instead of toward the centre (set in most records).
const FALLOFF_INVERTED: u32 = 0x100;
/// Fall-off amounts at or below this leave a surface alone.
const FALLOFF_MIN: f32 = 0.01;
/// Material flags (`0x48`): lit or specular. Without any, the emissive's
/// alpha is the opacity.
const SHADED: u32 = 0x7;
/// Material flag (`0x48`) of the emissive layer.
const EMISSIVE_LAYER: u32 = 0x8_0000;
/// Material extra flags (`0x4C`) choosing the dirt mode.
const DIRT_MODE: u32 = 0x7000;
/// The dirt mode that shows blood: the speckle texture against the vertex
/// alpha, whatever dirt map the record names.
const DIRT_MODE_BLOOD: u32 = 0x4000;

/// Whether a bleeding race's body-model material shows blood: the original
/// race loader turns dirt mode 4 on for the materials whose name starts
/// with `s` (`skinhuman`, `skinogre`, `sdemona01`, `shrtogra`,
/// `sknhrtghoula`, …), not the eyes.
fn takes_race_blood(material: &str) -> bool {
    material.starts_with('s')
}

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
        // Per-joint boxes, so the skinned mesh's box can follow its joints.
        let _ = m.generate_skinned_mesh_bounds();
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
    use bevy::render::render_resource::TextureFormat;

    fn flat(w: usize, v: u8) -> Texels {
        Texels {
            width: w,
            height: w,
            rgba: [v, v, v, 255].repeat(w * w),
        }
    }

    fn condition(quality: u8, damage: u8) -> Appearance {
        Appearance {
            quality,
            damage,
            dirt: 0,
            layers: Default::default(),
        }
    }

    /// The original's colour chain: the finishes read the shade map and
    /// library textures as stored and the palette colour as computed; their
    /// result is kept as an sRGB texel and decoded when sampled. Palette
    /// colour 1 (`#c7c7c7`, 0.78) on a mid-grey shade map (128):
    /// - a finish without its own painter paints 0.78 × 0.502 × 1.6 = 0.626,
    ///   texel 160, sampled as 0.3515. Decoding the shade map would paint
    ///   texel 69, linearising the colour 117, and uploading the texel as
    ///   linear data would sample 0.627;
    /// - rough cloth in the braies' condition (quality 160, damage 64) over
    ///   flat mid-grey library textures paints texel 96, sampled as 0.1170
    ///   (the original's cloth program gives the same for these inputs).
    #[test]
    fn painted_colour_samples_as_the_original_does() {
        let shade = flat(4, 128);
        let full = flat(4, 255);
        let grey = flat(4, 128);
        for (material, appearance, texel, sampled) in [
            (30, condition(255, 0), 160, 0.3515),
            (1, condition(160, 64), 96, 0.1170),
        ] {
            let layer = PaintLayer {
                mask: &full,
                material,
                colour: 1,
                library: vec![Some(&grey); 4],
                normals: vec![],
            };
            let painted = paint(&shade, None, &[layer], &appearance);
            let image = painted_image(4, 4, painted.colour, Painted::COLOUR);
            assert_eq!(
                image.texture_descriptor.format,
                TextureFormat::Rgba8UnormSrgb
            );
            let data = image.data.expect("pixels");
            assert_eq!(data[..3], [texel; 3], "material {material}");
            let decoded = linear(data[0] as f32 / 255.0);
            assert!(
                (decoded - sampled).abs() < 1e-3,
                "material {material}: {decoded}"
            );
        }
        let surface = painted_image(4, 4, vec![0; 64], Painted::SURFACE);
        assert_eq!(surface.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        let normal = painted_image(4, 4, vec![0; 64], Painted::NORMAL);
        assert_eq!(normal.texture_descriptor.format, TextureFormat::Rgba8Unorm);
    }

    /// An sRGB map's mips average colour in linear light: one white texel in
    /// four is a linear 0.25, texel 137, where averaging the stored values
    /// gives 64. Flat areas keep their texel; alpha is averaged as stored.
    #[test]
    fn srgb_mips_average_in_linear_light() {
        let px = |v: u8, a: u8| [v, v, v, a];
        let rgba = [px(0, 255), px(0, 255), px(0, 0), px(255, 0)].concat();
        let mip = |encoding| {
            painted_image(2, 2, rgba.clone(), encoding)
                .data
                .expect("pixels")[16..20]
                .to_vec()
        };
        assert_eq!(mip(Encoding::Srgb), [137, 137, 137, 128]);
        assert_eq!(mip(Encoding::Linear), [64, 64, 64, 128]);
        let flat = painted_image(2, 2, px(160, 255).repeat(4), Encoding::Srgb);
        assert_eq!(flat.data.expect("pixels")[16..20], [160, 160, 160, 255]);
    }

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

    fn record(class: u32, flags: u32, opacity: f32) -> Material {
        Material {
            class,
            flags,
            opacity,
            ..default()
        }
    }

    #[test]
    fn blended_cobwebs_cut_their_shadow() {
        // `cobweb01`: alpha-blended (source alpha, one minus source alpha),
        // opacity 1, the opacity flag and an opacity map.
        let cobweb = record(0x0203, 0x40_0017, 1.0);
        assert_eq!(blend_mode(cobweb.class, true), AlphaMode::Blend);
        assert!(cuts_shadow(&cobweb, true));
        // The flag alone (`harhps1`, no opacity layer), or a loaded opacity
        // map alone (records without the flag), is enough.
        assert!(cuts_shadow(&record(0x0203, 0x72_0007, 1.0), false));
        assert!(cuts_shadow(&record(0x0203, 0x22_0017, 1.0), true));
        // At or below 0.75 the shadow is whole (and at or below 0.8 there is
        // none at all).
        assert!(!cuts_shadow(&record(0x0203, 0x40_0017, 0.75), true));
        // Neither the flag nor an opacity map: a whole shadow.
        assert!(!cuts_shadow(&record(0x0203, 0x22_0017, 1.0), false));
        // Alpha-tested blend words always use the alpha-tested caster.
        assert!(cuts_shadow(&record(0x4100, 0x22_0017, 0.5), false));
    }

    #[test]
    fn salve_glow_is_additive_and_fades_toward_its_outline() {
        // `jarsm01glow`: unlit, emissive (0.235, 0.467, 0.627), opacity 0,
        // source alpha / one, fall-off amount 1 and exponent 2, extra 0x104.
        let glow = Material {
            emissive: [0.2353, 0.4667, 0.6275, 1.0],
            opacity: 0.0,
            class: 0x0001_0201,
            flags: 0x0008_0008,
            extra: 0x104,
            falloff_amount: 1.0,
            falloff_exponent: 2.0,
            ..default()
        };
        assert_eq!(blend_mode(glow.class, false), AlphaMode::Add);
        let e = emissive(&glow);
        // Unlit: no doubling, and the opacity as alpha.
        assert!(
            (e.truncate() - Vec3::new(0.0450, 0.1845, 0.3515))
                .abs()
                .max_element()
                < 1e-3
        );
        assert_eq!(e.w, 0.0);
        let (f, floor) = opacity_falloff(&glow).unwrap();
        assert_eq!((f.x, f.y), (1.0, -3.6));
        assert!((f.z - 1.0825).abs() < 1e-4);
        // (f + 1)·f with f = 2^−3.6.
        assert!((floor - 1.0825 * 0.0825).abs() < 1e-4);
        // A faint amount has none.
        let faint = Material {
            falloff_amount: 0.01,
            ..glow.clone()
        };
        assert!(opacity_falloff(&faint).is_none());
    }

    #[test]
    fn lit_emissive_maps_double_the_colour() {
        let handle = Material {
            emissive: [1.0, 0.5, 0.0, 1.0],
            flags: 0x0008_0003,
            ..default()
        };
        let e = emissive(&handle);
        assert!(
            (e.truncate() - Vec3::new(2.0, 2.0 * linear(0.5), 0.0))
                .abs()
                .max_element()
                < 1e-5
        );
        assert_eq!(e.w, 1.0);
        // Without an emissive layer the colour stays as it is.
        let plain = Material {
            flags: 0x3,
            ..handle.clone()
        };
        assert_eq!(emissive(&plain).x, 1.0);
    }

    #[test]
    fn race_blood_takes_the_skins_not_the_eyes() {
        for skin in [
            "skinhuman",
            "skinogre",
            "sdemona01",
            "shrtogra",
            "sknhrtghoula",
        ] {
            assert!(takes_race_blood(skin), "{skin}");
        }
        for other in [
            "eyehuman",
            "ogreeyemouth",
            "hrthogaeye",
            "golemgrda01",
            "Skin",
        ] {
            assert!(!takes_race_blood(other), "{other}");
        }
    }

    /// What the shipped rigid bodies' mobility is divided by (skipped
    /// without the game): the physical divisor of each body node's first
    /// material, most of them 1 (stored 0), iron 2 and 3, wicker 0.6; and
    /// the loaded models carry it.
    #[test]
    fn shipped_mobility_divisors() {
        use std::collections::BTreeMap;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        for (kind, bodies, want) in [
            (
                PackKind::Objects,
                1230,
                &[("0.5", 21), ("1", 1144), ("2", 33), ("3", 32)][..],
            ),
            (
                PackKind::Objlib,
                1503,
                &[
                    ("0.6", 11),
                    ("0.8", 2),
                    ("1", 1297),
                    ("10", 15),
                    ("2", 76),
                    ("3", 102),
                ][..],
            ),
        ] {
            let pack = game.pack(kind).unwrap();
            let mut divisors: BTreeMap<String, usize> = BTreeMap::new();
            let mut count = 0;
            for e in pack.entries() {
                let Ok(scene) = Scene::parse(pack.bytes(e)) else {
                    continue;
                };
                for n in scene.nodes.iter().filter(|n| n.rigid().is_some()) {
                    count += 1;
                    let NodeBody::Mesh(mesh) = &n.body else {
                        panic!("{}: a body off a mesh", e.name);
                    };
                    let name = &mesh.groups[0].material;
                    let m = scene
                        .material(name)
                        .cloned()
                        .or_else(|| library_material(&game, name))
                        .unwrap_or_else(|| panic!("{}: no material {name}", e.name));
                    *divisors
                        .entry(m.mobility_divisor().to_string())
                        .or_default() += 1;
                }
            }
            assert_eq!(count, bodies, "{kind:?}");
            let got: Vec<(&str, usize)> = divisors.iter().map(|(k, &v)| (k.as_str(), v)).collect();
            assert_eq!(got, want, "{kind:?}");
        }
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        // An iron anvil (`mtliron03`), a wicker basket (`wkrtl01`), a
        // terracotta amphora (stored 0).
        for (entry, divisor) in [
            ("anvil", 2.0),
            ("basket_wkr a", 0.6),
            ("amph trcb a01", 1.0),
        ] {
            let model = loader.model(PackKind::Objlib, entry).unwrap();
            let rigid = model.nodes.iter().find_map(|n| n.rigid.as_ref()).unwrap();
            assert_eq!(rigid.mobility_divisor, divisor, "{entry}");
            assert_eq!(rigid.mobility(), rigid.body.mobility / divisor, "{entry}");
        }
    }

    /// Which shipped materials show blood, and how (skipped without the
    /// game): the human skin as skin, the braies as cloth, a bleeding
    /// race's skin as cloth and not its eyes, nothing on the ancient's
    /// skin or a skeleton (which does not bleed).
    #[test]
    fn shipped_blood_materials() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let blood = |loader: &mut ModelLoader, entry: &str| {
            let model = loader.model(PackKind::Resource, entry).unwrap();
            let mut found: Vec<(String, Option<BloodProgram>)> = model
                .nodes
                .iter()
                .flat_map(|n| n.parts.iter().map(|p| p.1.clone()).zip(&n.part_materials))
                .map(|(h, name)| {
                    let m = loader.materials.get(&h).unwrap();
                    if m.blood().is_some() {
                        assert!(m.dirt_map.is_some(), "{entry} {name}");
                    }
                    (name.to_ascii_lowercase(), m.blood())
                })
                .collect();
            found.sort_by(|a, b| a.0.cmp(&b.0));
            found.dedup();
            found
        };
        assert_eq!(
            blood(&mut loader, "humanbase.rfc"),
            [
                ("eyehuman".into(), None),
                ("skinhuman".into(), Some(BloodProgram::Skin))
            ]
        );
        assert_eq!(
            blood(&mut loader, "ogrebase.rfc"),
            [
                ("ogreeyemouth".into(), None),
                ("skinogre".into(), Some(BloodProgram::Cloth))
            ]
        );
        assert_eq!(
            blood(&mut loader, "demona01base.rfc"),
            [("sdemona01".into(), Some(BloodProgram::Cloth))]
        );
        assert!(
            blood(&mut loader, "ancientbase.rfc")
                .iter()
                .all(|(_, b)| b.is_none())
        );
        assert!(
            blood(&mut loader, "skelbase.rfc")
                .iter()
                .all(|(_, b)| b.is_none())
        );
        let braies = Appearance {
            quality: 160,
            damage: 64,
            dirt: 0,
            layers: Default::default(),
        };
        let painted = loader.painted_apparel("braies__aa", &braies);
        let m = loader.materials.get(&painted).unwrap();
        assert_eq!(m.blood(), Some(BloodProgram::Cloth));
    }

    /// The speckle blood shows through (skipped without the game): texels
    /// at or above 0.5 cover about 13 % of `spatter01`, at or above 0.25
    /// 46 %, at or above 0.125 83 %. So a skin vertex painted to 128 shows
    /// blood as separate droplets on about an eighth of its texels.
    #[test]
    fn shipped_spatter_speckle() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let rfi = Rfi::parse(game.texture(SPATTER).expect("spatter01")).unwrap();
        assert_eq!((rfi.header.width, rfi.header.height), (1024, 1024));
        let rgba = rfi.decode_rgba8().unwrap();
        let red: Vec<f32> = rgba.iter().step_by(4).map(|&r| r as f32 / 255.0).collect();
        let share = |at: f32| red.iter().filter(|&&a| a >= at).count() as f32 / red.len() as f32;
        for (at, expected) in [(0.5, 0.13), (0.25, 0.46), (0.125, 0.83)] {
            let s = share(at);
            assert!((s - expected).abs() < 0.015, "{at}: {s}");
        }
        let alpha = 128.0 / 255.0;
        let bleeding = red
            .iter()
            .filter(|&&a| BloodProgram::Skin.amount(a, alpha) > 0.0)
            .count() as f32
            / red.len() as f32;
        assert!((bleeding - 0.13).abs() < 0.015, "{bleeding}");
    }

    /// How many material layers name an image the game does not ship, by
    /// the stored (cut and folded) name the original looks up: the original
    /// draws its 1x1 grey placeholder for each, Exarust no map (run with
    /// `cargo test -p exa_assets -- --ignored --nocapture` and the game
    /// installed). Most are in development scenes and apparel bases.
    #[test]
    #[ignore]
    fn shipped_materials_missing_images() {
        use exa_formats::FileKind;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let mut layers = 0usize;
        let mut missing: HashMap<(String, String), usize> = HashMap::new();
        let mut names: std::collections::BTreeSet<String> = Default::default();
        for (kind, pack) in game.packs() {
            for e in pack.entries() {
                let bytes = pack.bytes(e);
                if !matches!(
                    FileKind::detect(bytes),
                    FileKind::ChunkTree | FileKind::ChunkTreeAlt
                ) {
                    continue;
                }
                let Ok(scene) = Scene::parse(bytes) else {
                    continue;
                };
                for (_, m) in scene.materials {
                    let Ok(m) = m else { continue };
                    for layer in &m.layers {
                        layers += 1;
                        // The original looks the image up by its stored
                        // name: cut at the last dot, folded.
                        let stored = crate::texture_cache::TextureName::new(&layer.texture);
                        if game.texture(stored.as_str()).is_none() {
                            if layer.slot != TextureSlot::Reflection {
                                names.insert(format!("{kind:?}:{}", stored.as_str()));
                            }
                            *missing
                                .entry((format!("{kind:?}"), format!("{:?}", layer.slot)))
                                .or_default() += 1;
                        }
                    }
                }
            }
        }
        let mut rows: Vec<_> = missing.into_iter().collect();
        rows.sort();
        eprintln!("{layers} layers; missing images by pack and slot: {rows:?}");
        eprintln!(
            "{} distinct names outside reflections: {names:?}",
            names.len()
        );
    }

    /// The corpus records the rule was established on (run with
    /// `cargo test -p exa_assets -- --ignored` and the game installed).
    #[test]
    #[ignore]
    fn shipped_cut_shadow_records() {
        use exa_formats::FileKind;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let mut found = HashMap::new();
        'packs: for (_, pack) in game.packs() {
            for e in pack.entries() {
                let bytes = pack.bytes(e);
                if !matches!(
                    FileKind::detect(bytes),
                    FileKind::ChunkTree | FileKind::ChunkTreeAlt
                ) {
                    continue;
                }
                let Ok(scene) = Scene::parse(bytes) else {
                    continue;
                };
                for (name, m) in scene.materials {
                    if let Ok(m) = m
                        && (name.eq_ignore_ascii_case("cobweb01")
                            || name.eq_ignore_ascii_case("harhps1"))
                    {
                        found.insert(name.to_ascii_lowercase(), m);
                    }
                }
                if found.len() == 2 {
                    break 'packs;
                }
            }
        }
        let cobweb = &found["cobweb01"];
        assert_eq!((cobweb.class, cobweb.flags), (0x0203, 0x40_0017));
        assert_eq!(cobweb.opacity, 1.0);
        assert_eq!(cobweb.texture(TextureSlot::Opacity), Some("CobWb01O"));
        assert!(cuts_shadow(cobweb, true));
        // The opacity flag without an opacity layer.
        let harhps = &found["harhps1"];
        assert_ne!(harhps.flags & OPACITY_FLAG, 0);
        assert_eq!(harhps.texture(TextureSlot::Opacity), None);
        assert!(cuts_shadow(harhps, false));
    }
}
