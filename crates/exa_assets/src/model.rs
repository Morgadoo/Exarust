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
    Condition, Encoding, PaintLayer, Painted, Texels, Tiling, apparel_tint, factory_colour,
    factory_wear, layer_material, library, normal_library, paint_with, specular_library,
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
    /// Meshes of the node's `_null` groups: geometry the original never
    /// draws but its cursor pick still meets (the hub's shop counter box).
    /// Bare positions and indices, never uploaded; built as pick-only
    /// colliders. Empty for a skinned mesh.
    pub collision: Vec<Handle<Mesh>>,
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
    /// Whether a part of this node is the race's built-in skin (a body
    /// model's `skinhuman`), which a character's tone re-tints.
    pub is_skin: bool,
    /// Whether the node is an object-library placeholder
    /// ([`exa_formats::rfc::Node::PLACEHOLDER`]): a clone keeps its
    /// transform but draws nothing and has no body.
    pub placeholder: bool,
    /// The node carries the cloth marker (the mesh is simulated as cloth in
    /// the game: long hair, banners, the wraith's robe). The original makes
    /// a controller only in a scene carrying the cloth-flag 0x10000, which
    /// every game scene does ([`exa_formats::rfc::cloth_import`]).
    pub cloth: bool,
    /// The node stores a mesh body, whatever its groups draw.
    pub mesh: bool,
    /// The mesh body's render word, which the original copies onto its
    /// mesh node as the body is read
    /// ([`exa_formats::rfc::Mesh::render_flags`]). The dressed-mesh pass
    /// reads flag `0x40` to find overlay meshes and mark slot meshes.
    pub render_flags: u32,
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

/// A texture a material binds, with what the shading needs to know about
/// the upload: whether it carries alpha worth alpha-testing, and whether it
/// is a single grey channel (BC4) whose red stands for the colour of all
/// three channels.
#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub image: Handle<Image>,
    pub alpha: bool,
    pub grey: bool,
}

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
    /// The built-in race skin by body model, tone and sex
    /// ([`ModelLoader::skin_material_of`]).
    skin_materials: HashMap<(String, [u16; 2], bool), Handle<ExaMaterial>>,
    /// The composited skin materials by
    /// [`crate::skin_composite::SkinCompositeJob::key`].
    pub(crate) skin_composites: HashMap<String, Option<Handle<ExaMaterial>>>,
    /// Painted factory regions by [`FactoryPaint::key`].
    factory_painted: HashMap<String, Handle<ExaMaterial>>,
    /// Factory weapon stats once its assembled extents are known.
    pub(crate) factory_stats: HashMap<String, exa_formats::weapon_stats::StatBytes>,
    /// Parsed factory recipes, by lower-case name.
    pub(crate) factories: HashMap<String, Arc<exa_formats::factory::Factory>>,
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

impl ModelCache {
    /// Forgets every parsed, decoded and painted cache entry. A level change
    /// calls it after the old level's entities are gone: the assets they
    /// referenced (meshes, materials, textures) remain with the surviving
    /// player and are reused, while the level's parsed geometry and decoded
    /// texels are freed and the next level loads from the packs again.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The shading flags a diffuse slot adds from its texture word: a BC4 image
/// is one grey channel, and a word that skips the LOD bias makes the
/// material select a zero bias for its diffuse sample.
pub(crate) fn diffuse_shading_flags(word: u32, grey: bool) -> u32 {
    let mut flags = 0;
    if grey {
        flags |= flags::GREY_DIFFUSE;
    }
    if word & texture::flags::NO_LOD_BIAS != 0 {
        flags |= flags::SKIP_DIFFUSE_BIAS;
    }
    flags
}

/// The shading flag a specular or emissive slot adds when its image is a
/// single-channel BC4 upload: the shader must repeat the red channel, as it
/// does for a grey diffuse.
pub(crate) fn grey_slot_flag(grey: bool, flag: u32) -> u32 {
    if grey { flag } else { 0 }
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

    /// The first pack, in the project's object-library order, that holds
    /// `entry` (the original's "search each object library" step). The
    /// project names `Objlib`; a project with more libraries searches them
    /// in its order.
    pub fn object_pack(&self, entry: &str) -> Option<PackKind> {
        self.game
            .project()
            .object_libraries
            .iter()
            .filter_map(|name| PackKind::from_name(name))
            .find(|&pack| self.game.pack(pack).is_some_and(|p| p.get(entry).is_some()))
    }

    /// The model of an object-library entry, searched from the project's
    /// object libraries ([`ModelLoader::object_pack`]).
    pub fn object_model(&mut self, entry: &str) -> Option<Arc<Model>> {
        let pack = self.object_pack(entry)?;
        self.model(pack, entry)
    }

    /// Forgets a cached model, as the original's object-lib release drops
    /// the master when its last user lets go. The materials and textures
    /// it loaded stay cached (other models may share them, and the original
    /// keeps its libraries too); a later request loads the model again.
    pub fn release_model(&mut self, pack: PackKind, entry: &str) {
        self.cache.models.remove(&model_key(pack, entry));
        let smooth = format!("{}#smooth", model_key(pack, entry));
        self.cache.models.remove(&smooth);
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

    /// The model of `entry` built from its file after `edit` changes the
    /// parsed scene (a level placement's node overrides, which the original
    /// applies to its copy of the object before it builds the render
    /// meshes). Cached under `variant`, apart from the plain model; the
    /// model's nodes are the scene's, in order.
    pub fn edited_model(
        &mut self,
        pack: PackKind,
        entry: &str,
        variant: &str,
        edit: impl FnOnce(&mut Scene),
    ) -> Option<Arc<Model>> {
        let key = format!("{}#{variant}", model_key(pack, entry));
        if let Some(cached) = self.cache.models.get(&key) {
            return cached.clone();
        }
        let model = self.model_job(pack, entry).map(|mut job| {
            edit(&mut job.scene);
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
            let mesh = match &node.body {
                NodeBody::Mesh(mesh) => Some(mesh.as_ref()),
                _ => None,
            };
            if let Some(mesh) = mesh
                && mesh.indices.is_empty()
            {
                // The original's after-load init reports a mesh without
                // faces and keeps it empty.
                self.warn_once(format!("{}:{}: mesh has no faces", entry, node.name));
            }
            let skin_bone = built.skin.as_ref().map(|(bone, _)| *bone);
            // A node marked as level geometry never registers its body.
            let rigid = node
                .rigid()
                .filter(|_| !node.is_static())
                .map(|body| NodeRigidBody {
                    body: Arc::new(body.clone()),
                    mobility_divisor: mesh.map_or(1.0, |mesh| {
                        mesh.groups
                            .first()
                            .map_or(1.0, |g| self.mobility_divisor(&entry, &scene, &g.material))
                    }),
                });
            let (parts, part_materials, collision) = match mesh {
                Some(mesh) => {
                    let (parts, part_materials) = built
                        .meshes
                        .into_iter()
                        .map(|(group, bevy_mesh)| {
                            let name = &mesh.groups[group].material;
                            let material = self.material(&entry, &scene, name);
                            ((self.meshes.add(bevy_mesh), material), name.clone())
                        })
                        .unzip();
                    let collision = built
                        .collision
                        .into_iter()
                        .map(|mesh| self.meshes.add(mesh))
                        .collect();
                    (parts, part_materials, collision)
                }
                None => (Vec::new(), Vec::new(), Vec::new()),
            };
            let is_skin = match &node.body {
                NodeBody::Mesh(mesh) => mesh
                    .groups
                    .iter()
                    .any(|g| crate::human_skin::is_race_skin(&entry, &g.material)),
                _ => false,
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
                collision,
                skin_bone,
                bounds: (Vec3::from(node.bounds_centre), node.bounds_radius),
                half_extents: Vec3::from(node.bounds_half_extents),
                gi_points: Arc::new(match mesh {
                    Some(mesh) if skin_bone.is_none() => {
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
                shadow_bias: match mesh {
                    // The inflated variant (both sides × (1 + 0.005·v)) is
                    // not reproduced: such a mesh keeps level 0.
                    Some(mesh) if !mesh.shadow_bias_inflated() => mesh.shadow_bias_level(),
                    _ => 0,
                },
                is_skin,
                placeholder: node.flags & exa_formats::rfc::Node::PLACEHOLDER != 0,
                cloth: exa_formats::rfc::cloth_import(
                    node.cloth_marker,
                    exa_formats::rfc::CLOTH_SCENE,
                ),
                mesh: mesh.is_some(),
                render_flags: mesh.map_or(0, |mesh| mesh.render_flags),
            });
        }
        // An instance node shares the node it names (1-based): the original
        // links it to that node's object, so it draws the same geometry,
        // light or emitter at its own transform and parent.
        for i in 0..nodes.len() {
            if scene.nodes[i].instance.is_none() {
                continue;
            }
            let Some(owner) = instance_root(&scene, i) else {
                continue;
            };
            if owner == i || owner >= nodes.len() {
                continue;
            }
            let (
                parts,
                part_materials,
                collision,
                gi_points,
                shadow_bias,
                light,
                voxel_light,
                fire,
                portal,
            ) = {
                let src = &nodes[owner];
                (
                    src.parts.clone(),
                    src.part_materials.clone(),
                    src.collision.clone(),
                    src.gi_points.clone(),
                    src.shadow_bias,
                    src.light,
                    src.voxel_light,
                    src.fire,
                    src.portal,
                )
            };
            let dst = &mut nodes[i];
            dst.parts = parts;
            dst.part_materials = part_materials;
            dst.collision = collision;
            dst.gi_points = gi_points;
            dst.shadow_bias = shadow_bias;
            dst.light = light;
            dst.voxel_light = voxel_light;
            dst.fire = fire;
            dst.portal = portal;
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
        // The original's material manager wins over a file's own list: the
        // reader adds a file's material only when the manager has no record
        // of that name, so a library material shades the surface.
        let source = library_material(self.game, name).or(local);
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
                let handle = self.builtin_default_material();
                self.cache.materials.insert(key, handle.clone());
                return handle;
            }
        };
        let handle = self.materials.add(mat);
        self.cache.materials.insert(key, handle.clone());
        handle
    }

    /// The built-in `_Default` material every unknown name resolves to,
    /// shaded from its stored parameter block and created once: the original
    /// keeps a single `_Default` object and counts its users.
    fn builtin_default_material(&mut self) -> Handle<ExaMaterial> {
        let key = format!(
            "lib#{}",
            exa_formats::material::DEFAULT_MATERIAL_NAME.to_ascii_lowercase()
        );
        if let Some(h) = self.cache.materials.get(&key) {
            return h.clone();
        }
        let shaded = self.exa_material(&Material::builtin_default());
        let handle = self.materials.add(shaded);
        self.cache.materials.insert(key, handle.clone());
        handle
    }

    /// A material shaded like the original's from its library record.
    fn exa_material(&mut self, m: &Material) -> ExaMaterial {
        let packed = m.kind & 1 != 0;
        let key = exa_render::shader_program::feature_key(m);
        let lit = key & material_flags::LIT != 0;
        let state_word =
            exa_render::shader_program::state_words(m, SHADING_QUALITY).map_or(0, |(mat, _)| mat);
        let maps = factory_maps(m, key, state_word);
        let diffuse = shaded_diffuse(m, key);
        let specular = material_specular(m);
        let mut shading = 0;
        if lit {
            shading |= flags::LIT;
        }
        if key & material_flags::TWO_SIDED != 0 {
            shading |= flags::TWO_SIDED;
        }
        if packed {
            shading |= flags::PACKED_SPECULAR;
        }
        let mut mat = ExaMaterial {
            params: ExaParams {
                diffuse: diffuse.extend(m.opacity),
                specular: specular.extend(material_gloss(m)),
                emissive: emissive(m, key),
                flags: shading,
                ..default()
            },
            ..default()
        };
        mat.params.texture_bias = self.cache.textures.settings().lod_bias();
        // The record's texture load flags, with the mode bits the original
        // adds per slot: normal, gloss and height maps (and the specular map
        // of a material whose kind has bit 0) are data, and a diffuse with a
        // secondary colour skips the LOD bias.
        let settings = *self.cache.textures.settings();
        let base_flags = material_word(m.texture_flags, &settings);
        let data_flags = base_flags | texture::flags::DATA | texture::flags::NO_LOD_BIAS;
        let bias_skipped = base_flags | texture::flags::NO_LOD_BIAS;
        let mut cut_out = false;
        if maps.diffuse
            && let Some(t) = m.texture(TextureSlot::Diffuse)
        {
            let flags = if m.has_secondary_colour() {
                bias_skipped
            } else {
                base_flags
            };
            if let Some(info) = self.texture_info(t, ColorSpace::Srgb, flags) {
                mat.diffuse_map = Some(info.image);
                cut_out = info.alpha;
                mat.params.flags |= diffuse_shading_flags(flags, info.grey);
            }
        }
        if maps.normal
            && let Some(t) = m.texture(TextureSlot::Normal)
        {
            mat.normal_map = self
                .texture_with_flags(t, ColorSpace::Linear, data_flags)
                .map(|t| t.0);
        }
        if maps.specular
            && let Some(t) = m.texture(TextureSlot::Specular)
        {
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
            if let Some(info) = self.texture_info(t, space, flags) {
                mat.specular_map = Some(info.image);
                mat.params.flags |= grey_slot_flag(info.grey, flags::GREY_SPECULAR);
            }
        }
        if maps.emissive
            && let Some(t) = m.texture(TextureSlot::Emissive)
            && let Some(info) = self.texture_info(t, ColorSpace::Srgb, base_flags)
        {
            mat.emissive_map = Some(info.image);
            mat.params.flags |= grey_slot_flag(info.grey, flags::GREY_EMISSIVE);
        }
        if maps.opacity
            && let Some(t) = m.texture(TextureSlot::Opacity)
        {
            mat.opacity_map = self
                .texture_with_flags(t, ColorSpace::Linear, base_flags)
                .map(|t| t.0);
        }
        // Dirt maps only act through vertex colours, which level meshes lack.
        if maps.gloss
            && let Some(t) = m.texture(TextureSlot::Gloss)
        {
            mat.gloss_map = self
                .texture_with_flags(t, ColorSpace::Linear, data_flags)
                .map(|t| t.0);
        }
        // Parallax: the height layer's depth as a fixed number of texels of
        // the height map (loaded now: its size is the decoded image's); displacement shadows when the record asks.
        if maps.height
            && let Some(layer) = m.layer(TextureSlot::Height)
            && layer.amount > 0.0
            && let Some((img, _)) = self.texture_now(&layer.texture, ColorSpace::Linear, data_flags)
            && let Some(size) = self.images.get(&img).map(|i| i.size())
            && size.x > 1
            && size.y > 1
        {
            let (x, y) = exa_render::shader_uniforms::parallax_texels(layer.amount, size.x, size.y);
            mat.params.displacement = Vec4::new(
                f32::from(x) * PARALLAX_UNIT,
                f32::from(y) * PARALLAX_UNIT,
                m.displacement_step * 0.004,
                m.displacement_bias * 0.05,
            );
            mat.params.flags |= flags::HEIGHT_MAP;
            if m.extra & DISPLACEMENT_SHADOWS != 0 {
                mat.params.flags |= flags::DISPLACEMENT_SHADOWS;
            }
            mat.height_map = Some(img);
        }
        mat.alpha_mode = blend_mode(blended_class(m), cut_out || mat.opacity_map.is_some());
        if mat.alpha_mode == AlphaMode::Add {
            mat.params.flags |= flags::ADDITIVE;
        }
        // Faint surfaces (glass, decals) and those the record marks cast no
        // shadows; a refracting record casts, and one whose destination
        // factor is ONE never does.
        mat.no_shadows = !casts_shadow(m);
        mat.cut_shadow = cuts_shadow(m, mat.opacity_map.is_some());
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
        // The cube stays bound (it lights every surface outside the GI
        // volume), but only a record whose material state keeps its
        // reflection layer (`Mat.TexRef`) reflects it inside the volume;
        // every other surface's specular GI there is a quarter of the
        // ambient. The original loads that layer's own cube; Exarust binds
        // the level's.
        mat = mat.with_map_flags();
        if !reflects_cube(m) {
            mat.params.flags &= !flags::ENVIRONMENT;
        }
        mat
    }

    /// The material of a run-time (`&`) apparel texture set without its
    /// painting, as the original's `_procmat1` built-in replaces it: blood
    /// shows on it as on cloth.
    fn apparel_material(&mut self, texture_key: &str) -> Handle<ExaMaterial> {
        let mut mat = ExaMaterial {
            environment: self.environment_map(),
            ..default()
        };
        mat.params.texture_bias = self.cache.textures.settings().lod_bias();
        let flags = texture::default_flags();
        match self.texture_info(&format!("{texture_key}d"), ColorSpace::Srgb, flags) {
            Some(TextureInfo { image, alpha, grey }) => {
                mat.diffuse_map = Some(image);
                mat.params.flags |= diffuse_shading_flags(flags, grey);
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

    /// The material of a factory weapon's component region, painted as the
    /// original's factory paint job does: each mask with its area's
    /// material, lifted colour and finish level, the item's condition with
    /// the factory's remapping, and the region's tiling.
    pub fn painted_factory(&mut self, paint: &FactoryPaint) -> Handle<ExaMaterial> {
        let key = paint.key();
        if let Some(h) = self.cache.factory_painted.get(&key) {
            return h.clone();
        }
        let stem = paint.stem.to_ascii_lowercase();
        let Some(shade) = self.texels(&format!("{stem}d")) else {
            let handle = self.apparel_material(&stem);
            self.cache.factory_painted.insert(key, handle.clone());
            return handle;
        };
        let mut layers: Vec<OwnedLayer> = Vec::new();
        for layer in &paint.layers {
            let mask_name = format!("{stem}m{}", layer.mask);
            if self.game.texture(&mask_name).is_none() {
                // The original stops at the first missing mask.
                break;
            }
            let Some(mask) = self.texels(&mask_name) else {
                break;
            };
            // An area without a material paints as SteelGen.
            let material = if layer.material == 0 {
                12
            } else {
                layer.material
            };
            let (lib, spec, normals) = self.material_texels(material);
            layers.push(OwnedLayer {
                mask,
                material,
                tint: factory_colour(layer.colour),
                wear: factory_wear(layer.finish),
                library: lib,
                specular: spec,
                normals,
            });
        }
        let item_normal = self.texels(&format!("{stem}n"));
        let painted_size = Vec2::new(shade.width as f32, shade.height as f32);
        let job = PaintJob {
            key: PaintKey::Factory(stem, key.clone()),
            shade,
            item_normal,
            layers,
            condition: Condition::factory(&paint.condition),
            tiling: Tiling::factory(paint.region_texels, painted_size, paint.seed_b),
        };
        let painted = job.paint();
        self.painted_material(painted)
    }

    /// The library textures a material's three passes read, decoded.
    fn material_texels(&mut self, material: u8) -> (TexelList, TexelList, TexelList) {
        let lib = library(material).iter().map(|n| self.texels(n)).collect();
        let spec = specular_library(material)
            .iter()
            .map(|n| self.texels(n))
            .collect();
        let normals = normal_library(material)
            .iter()
            .map(|n| self.texels(n))
            .collect();
        (lib, spec, normals)
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
            // A layer whose mask the set lacks ends the list: the original
            // stops reading there.
            if self.game.texture(&mask).is_none() {
                break;
            }
            names.push(mask);
            let material = layer_material(layer);
            names.extend(library(material).iter().map(|n| n.to_string()));
            names.extend(specular_library(material).iter().map(|n| n.to_string()));
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
        self.paint_job_with(stem, appearance, Condition::from(appearance))
    }

    fn paint_job_with(
        &mut self,
        stem: &str,
        appearance: &Appearance,
        condition: Condition,
    ) -> Paint {
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
            // A layer whose mask the set lacks ends the list, as the
            // original's job stops reading there.
            let mask_name = format!("{stem}m{}", layer.mask);
            if self.game.texture(&mask_name).is_none() {
                break;
            }
            let Some(mask) = self.texels(&mask_name) else {
                continue;
            };
            let material = layer_material(layer);
            let (lib, spec, normals) = self.material_texels(material);
            let (tint, wear) = apparel_tint(material, layer.colour);
            layers.push(OwnedLayer {
                mask,
                material,
                tint,
                wear,
                library: lib,
                specular: spec,
                normals,
            });
        }
        let item_normal = self.texels(&format!("{stem}n"));
        Paint::Job(PaintJob {
            key: PaintKey::Apparel(key),
            shade,
            item_normal,
            layers,
            condition,
            tiling: Tiling::default(),
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
        mat.params.texture_bias = self.cache.textures.settings().lod_bias();
        if alpha {
            mat.alpha_mode = AlphaMode::Mask(0.5);
            mat.params.flags |= flags::TWO_SIDED;
        }
        mat.normal_map = match normal {
            Some(image) => Some(self.images.add(image)),
            None => self
                .texture(&format!("{}n", key.stem()), ColorSpace::Linear)
                .map(|t| t.0),
        };
        // The original's painted apparel (`_procmat1`) and factory items
        // (`_procmatr`) show blood as cloth.
        let spatter = self.spatter();
        let handle = self
            .materials
            .add(mat.with_blood(BloodProgram::Cloth, spatter));
        match key {
            PaintKey::Apparel(key) => self.cache.painted.insert(key, handle.clone()),
            PaintKey::Factory(_, key) => self.cache.factory_painted.insert(key, handle.clone()),
        };
        handle
    }

    /// The built-in skin of a body model at a character's tone
    /// ([`crate::human_skin::tone_colour`]) and sex, cached by them.
    pub fn skin_material_of(
        &mut self,
        entry: &str,
        tone: [u16; 2],
        female: bool,
    ) -> Handle<ExaMaterial> {
        let key = (entry.to_ascii_lowercase(), tone, female);
        if let Some(handle) = self.cache.skin_materials.get(&key) {
            return handle.clone();
        }
        let colour = crate::human_skin::tone_colour(tone);
        let material = crate::human_skin::skin_material_of(self, entry, colour, female);
        let handle = self.materials.add(material);
        self.cache.skin_materials.insert(key, handle.clone());
        handle
    }

    /// The environment cube the original lights and reflects with
    /// (`genref01r` indoors): six faces with their mip chains, sRGB.
    pub fn environment_map(&mut self) -> Option<Handle<Image>> {
        if let Some(h) = &self.cache.environment {
            return h.clone();
        }
        let settings = *self.cache.textures.settings();
        let image = self
            .game
            .texture(ENVIRONMENT_MAP)
            .and_then(|b| Rfi::parse(b).ok())
            .and_then(|rfi| {
                rfi_to_texture(
                    &rfi,
                    ColorSpace::Srgb,
                    self.bc_supported,
                    texture::default_flags(),
                    &settings,
                )
                .ok()
            });
        if image.is_none() {
            self.warn_once(format!(
                "environment map {ENVIRONMENT_MAP} could not be read"
            ));
        }
        let handle = image.map(|i| self.images.add(i));
        self.cache.environment = Some(handle.clone());
        handle
    }

    /// The image handle, its alpha flag and whether it is a single grey
    /// channel, with the creation/sampler word a material asks for.
    /// [`ModelLoader::texture_with_flags`]'
    /// [`crate::texture_cache::TextureRecord`] carries the flags, so they
    /// are known before a streamed upload arrives.
    pub fn texture_info(
        &mut self,
        name: &str,
        space: ColorSpace,
        flags: u32,
    ) -> Option<TextureInfo> {
        let (image, alpha) = self.texture_with_flags(name, space, flags)?;
        let grey = self
            .cache
            .textures
            .get(&TextureKey::new(name, space))
            .is_some_and(|record| record.grey);
        Some(TextureInfo { image, alpha, grey })
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
    /// With the manager's streaming path on ([`crate::texture_cache::TextureCache::set_streaming`])
    /// a new record gets the original's 1×1 grey placeholder and a load in
    /// the loader's queue; the loader's upload replaces the image under the
    /// same handle, so a material that binds it draws the placeholder until
    /// then. Otherwise the image is decoded here, on the calling thread.
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
        let streaming = self.cache.textures.streaming();
        self.texture_load(name, space, flags, streaming)
    }

    /// Releases a texture's reference and, when it was the last, frees its
    /// image: the original's manager frees the GL texture at zero
    /// references. `true` when the record went.
    pub fn release_texture(&mut self, name: &str, space: ColorSpace) -> bool {
        let key = TextureKey::new(name, space);
        match self.cache.textures.release_record(&key) {
            Some(record) => {
                if let Some(image) = record.image {
                    self.images.remove(image.id());
                }
                true
            }
            None => false,
        }
    }

    /// Loads a texture now, on the calling thread: the original's priority
    /// path (its synchronous create, or its "load this one first" wait). A
    /// load still queued for the loader is dropped, so it cannot upload a
    /// second copy; one already being decoded writes the same pixels under
    /// the record's handle, which is harmless.
    pub fn texture_now(&mut self, name: &str, space: ColorSpace, flags: u32) -> CachedTexture {
        let key = TextureKey::new(name, space);
        self.cache.textures.cancel_pending(&key);
        self.texture_load(name, space, flags, false)
    }

    /// [`ModelLoader::texture_now`] with the default texture word: for the
    /// consumers that cannot take a placeholder (terrain and particle
    /// materials, whose bindings the loader's upload does not refresh).
    pub fn texture_sync(&mut self, name: &str, space: ColorSpace) -> CachedTexture {
        self.texture_now(name, space, crate::texture::default_flags())
    }

    fn texture_load(
        &mut self,
        name: &str,
        space: ColorSpace,
        flags: u32,
        streaming: bool,
    ) -> CachedTexture {
        let key = TextureKey::new(name, space);
        let created = self.cache.textures.acquire(key, flags).created;
        // A record whose image is still the loader's placeholder is decoded
        // here when the caller wants it now (`texture_now`); with streaming
        // on, the placeholder stays and the loader replaces it.
        let placeholder = self
            .cache
            .textures
            .get(&key)
            .is_some_and(|record| record.pending);
        if created || (placeholder && !streaming) {
            let settings = *self.cache.textures.settings();
            let Some(bytes) = self.game.texture(key.name.as_str()) else {
                self.warn_once(format!("texture {name:?} not found"));
                return None;
            };
            if let Err(e) = crate::image_format::reader_for(key.name.as_str(), bytes) {
                self.warn_once(format!("texture {name:?}: {e}"));
                let image = crate::texture::placeholder_image(space, flags, &settings);
                let handle = self.store_texture(key, image, false);
                return Some((handle, false));
            }
            let rfi = match Rfi::parse(bytes) {
                Ok(rfi) => rfi,
                Err(e) => {
                    self.warn_once(format!("texture {name:?}: {e}"));
                    let image = crate::texture::placeholder_image(space, flags, &settings);
                    let handle = self.store_texture(key, image, false);
                    return Some((handle, false));
                }
            };
            let alpha = has_alpha(&rfi);
            self.cache.textures.set_grey(
                &key,
                crate::texture::is_grey_upload(&rfi, self.bc_supported),
            );
            if streaming {
                // The record draws its placeholder until the loader's upload
                // arrives under the same handle.
                let image = crate::texture::placeholder_image(space, flags, &settings);
                let handle = self.store_texture(key, image, alpha);
                self.cache.textures.set_pending(&key, true);
                self.cache
                    .textures
                    .queue(crate::stream::TextureRequest::new(
                        key.name.as_str(),
                        space,
                        flags,
                        key.name.as_str(),
                    ));
                return Some((handle, alpha));
            }
            match rfi_to_texture(&rfi, space, self.bc_supported, flags, &settings) {
                Ok(image) => {
                    let handle = self.store_texture(key, image, alpha);
                    Some((handle, alpha))
                }
                Err(e) => {
                    self.warn_once(format!("texture {name:?}: {e}"));
                    let image = crate::texture::placeholder_image(space, flags, &settings);
                    let handle = self.store_texture(key, image, false);
                    Some((handle, false))
                }
            }
        } else {
            self.cache
                .textures
                .get(&key)
                .and_then(|record| record.image.clone().map(|image| (image, record.alpha)))
        }
    }

    /// Stores a decoded image under the record's handle when it has one (a
    /// placeholder being replaced) and in a new asset otherwise, then marks
    /// the record finished.
    fn store_texture(&mut self, key: TextureKey, image: Image, alpha: bool) -> Handle<Image> {
        let existing = self
            .cache
            .textures
            .get(&key)
            .and_then(|record| record.image.clone());
        match existing {
            Some(handle) => {
                let _ = self.images.insert(handle.id(), image);
                self.cache.textures.finish(&key, handle.clone(), alpha);
                handle
            }
            None => {
                let handle = self.images.add(image);
                self.cache.textures.finish(&key, handle.clone(), alpha);
                handle
            }
        }
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
    /// The node's `_null` groups' bare meshes, never drawn.
    collision: Vec<Mesh>,
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
        let skins_of = |scene: &Scene| scene_skins(scene, skinned, pack);
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
                        collision: Vec::new(),
                    };
                };
                let (meshes, collision) =
                    group_meshes(mesh, skin.as_ref().map(|(_, s)| s.as_slice()), pool);
                BuiltNode {
                    skin,
                    meshes,
                    collision,
                }
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

/// The node whose data an instance node shares: follow the instance chain to
/// the first node that stores a body of its own (the owner words are 1-based
/// node indices).
fn instance_root(scene: &Scene, index: usize) -> Option<usize> {
    let mut at = index;
    for _ in 0..=scene.nodes.len() {
        let node = scene.nodes.get(at)?;
        if node.instance.is_none() {
            return Some(at);
        }
        let owner = node.instance?.checked_sub(1)?;
        if owner >= scene.nodes.len() || owner == at {
            return None;
        }
        at = owner;
    }
    None
}

/// Smooths a mesh node in place and refits its bounds; a mesh that cannot
/// be smoothed is left as it is, with the original's warnings.
fn smooth_node(node: &mut Node, flags: &[u32]) {
    let NodeBody::Mesh(mesh) = &mut node.body else {
        return;
    };
    match smooth_mesh(mesh, flags) {
        Ok(smoothed) => {
            if !smoothed.subdivided {
                warn!("MeshSmooth: {} has too many faces to subdivide", node.name);
            }
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
        Err(e) => warn!("MeshSmooth: {}: {e}", node.name),
    }
}

/// The skin weights of every node of a scene, when it is skinned at all.
pub(crate) fn scene_skins(
    scene: &Scene,
    skinned: bool,
    pack: PackKind,
) -> Vec<Option<(u16, Vec<VertexSkin>)>> {
    scene
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| match &node.body {
            NodeBody::Mesh(mesh) if skinned => node_skin(scene, index, mesh, pack),
            _ => None,
        })
        .collect()
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
) -> (Vec<(usize, Mesh)>, Vec<Mesh>) {
    let is_null = |g: usize| mesh.groups[g].material.eq_ignore_ascii_case("_null");
    let groups: Vec<usize> = (0..mesh.groups.len()).filter(|&g| !is_null(g)).collect();
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
    let drawn = built.into_iter().flatten().collect();
    // `_null` marks geometry the original never draws but its cursor pick
    // still meets, such as the hub's shop counter box: keep it as bare
    // pick meshes. A skinned mesh's `_null` seams are never static scenery.
    let collision = match skin {
        Some(_) => Vec::new(),
        None => (0..mesh.groups.len())
            .filter(|&g| is_null(g))
            .filter_map(|g| {
                let group = &mesh.groups[g];
                let start = group.first_index as usize;
                mesh.indices
                    .get(start..start + group.index_count as usize)
                    .and_then(|indices| pick_mesh(mesh, indices))
            })
            .collect(),
    };
    (drawn, collision)
}

/// A `_null` group's triangles as a bare mesh: positions and indices only,
/// kept in the main world for the static collider build and never uploaded.
fn pick_mesh(mesh: &RfcMesh, indices: &[u32]) -> Option<Mesh> {
    let mut remap = HashMap::with_capacity(indices.len());
    let mut positions = Vec::new();
    let mut local = Vec::with_capacity(indices.len());
    for &i in indices {
        let position = *mesh.positions.get(i as usize)?;
        local.push(*remap.entry(i).or_insert_with(|| {
            positions.push(position);
            (positions.len() - 1) as u32
        }));
    }
    if local.len() < 3 {
        return None;
    }
    let mut built = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD,
    );
    built.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    built.insert_indices(Indices::U32(local));
    Some(built)
}

/// A layer's mask, material, colour and library textures, owned while painting.
/// Decoded library textures of one pass, in the material's order.
type TexelList = Vec<Option<Arc<Texels>>>;

/// A paint layer with its textures shared from the decoded-texture cache.
struct OwnedLayer {
    mask: Arc<Texels>,
    material: u8,
    tint: Vec3,
    wear: f32,
    library: TexelList,
    specular: TexelList,
    normals: TexelList,
}

/// Which cache a painted set belongs to.
#[derive(Debug, Clone)]
enum PaintKey {
    /// Apparel: texture stem and appearance.
    Apparel((String, Appearance)),
    /// A factory region: its stem and [`FactoryPaint::key`].
    Factory(String, String),
}

impl PaintKey {
    fn stem(&self) -> &str {
        match self {
            PaintKey::Apparel((stem, _)) | PaintKey::Factory(stem, _) => stem,
        }
    }
}

/// One material area over a factory component's mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryLayer {
    /// Mask number (`<stem>m<n>`).
    pub mask: u8,
    pub material: u8,
    /// The area's finish byte (two levels; the paint takes the low one).
    pub finish: u8,
    /// Palette index.
    pub colour: u8,
}

/// What a factory region paints with.
#[derive(Debug, Clone, PartialEq)]
pub struct FactoryPaint {
    /// The component's texture stem.
    pub stem: String,
    /// The item's quality, damage and dirt bytes (its appearance's layers
    /// are unused).
    pub condition: Appearance,
    /// The areas over masks 1–4, in order, up to the first missing mask.
    pub layers: Vec<FactoryLayer>,
    /// The region's size in atlas texels (the atlas size times the part's
    /// atlas scale).
    pub region_texels: Vec2,
    /// The item's seed B, which places the library tiles.
    pub seed_b: u32,
}

impl FactoryPaint {
    /// The cache key: everything the painting depends on.
    pub fn key(&self) -> String {
        let c = &self.condition;
        format!(
            "{}|{},{},{}|{:?}|{},{}|{}",
            self.stem.to_ascii_lowercase(),
            c.quality,
            c.damage,
            c.dirt,
            self.layers,
            self.region_texels.x,
            self.region_texels.y,
            self.seed_b
        )
    }
}

/// A record's emissive term as the shader takes it (the original's MatEmi):
/// the linear colour, doubled while the type keeps `MatEmi` (`key & 8`) on
/// a lit or specular record whose type also keeps the emissive layer
/// (`key & 0x80000`); alpha the opacity for an emissive-only record, the
/// colour's own alpha otherwise. `key` is [`exa_render::shader_program::feature_key`],
/// the flags masked by the material type.
fn emissive(m: &Material, key: u32) -> Vec4 {
    let mut rgb = linear_rgb([m.emissive[0], m.emissive[1], m.emissive[2]]);
    if key & 0x8 != 0 && key & SHADED != 0 && key & EMISSIVE_LAYER != 0 {
        rgb *= 2.0;
    }
    let alpha = if key & 0x8 != 0 && key & SHADED == 0 {
        m.opacity
    } else {
        m.emissive[3]
    };
    rgb.extend(alpha)
}

/// Which of Exarust's texture maps the original's material factory would
/// bind for a record. Every map needs the layer and the feature bit its
/// type's plug-in keeps; the normal map also needs a surface the factory
/// shades (`0x100807`), and the height map needs the normal layer loaded. The
/// colour slot binds only a lit record's diffuse map (`key & 3`); an unlit
/// record's diffuse layer is left unbound.
/// A type-2 record keeps only `MatEmi`, two-sided and the colour/opacity
/// slot, so nothing but `opacity` can be set for it.
struct FactoryMaps {
    diffuse: bool,
    opacity: bool,
    emissive: bool,
    specular: bool,
    normal: bool,
    gloss: bool,
    height: bool,
}

/// The factory's map bindings for a record with feature word `key` and
/// material word `material_word` (see [`FactoryMaps`]).
fn factory_maps(m: &Material, key: u32, material_word: u32) -> FactoryMaps {
    FactoryMaps {
        diffuse: key & 0x2_0000 != 0
            && key & material_flags::LIT != 0
            && m.texture(TextureSlot::Diffuse).is_some(),
        opacity: key & OPACITY_FLAG != 0 && m.texture(TextureSlot::Opacity).is_some(),
        emissive: key & 0x8_0000 != 0
            && key & 0x8 != 0
            && m.texture(TextureSlot::Emissive).is_some(),
        specular: key & 0x4_0000 != 0
            && key & 0x10_0004 != 0
            && m.texture(TextureSlot::Specular).is_some(),
        normal: key & 0x20_0000 != 0
            && exa_render::shader_program::wants_normal_map(material_word)
            && m.texture(TextureSlot::Normal).is_some(),
        gloss: key & 0x100_0000 != 0 && m.texture(TextureSlot::Gloss).is_some(),
        height: key & 0x80_0000 != 0
            && m.texture(TextureSlot::Height).is_some()
            && m.texture(TextureSlot::Normal).is_some(),
    }
}

/// The blend word after the material loader's late opaque rewrite: a
/// record whose loaded destination nibble is 3 (the transparent class)
/// becomes source ONE / destination ZERO when it is in refraction mode
/// (loaded flags `0x300` = `0x200`, whatever its type), or when its opacity
/// is above 0.99 in double precision, its fall-off amount below 0.01 and its
/// loaded flags lack the opacity layer (`0x400000`, which a record's own
/// flag or a loaded opacity layer sets). The rewrite starts from the class
/// the alpha-test rewrite left ([`exa_render::shader_program::loaded_class`])
/// and NaN fails both comparisons, as the original's COMISD branches do. The
/// comparisons widen the stored f32 to f64: `0.99` stored as f32 is above
/// the bound and `0.01` is below it.
fn blended_class(m: &Material) -> u32 {
    let mut class = exa_render::shader_program::loaded_class(m);
    let flags = exa_render::shader_program::loaded_flags(m);
    if class & 0xF == 3
        && (flags & MODE_MASK == REFRACTION_MODE
            || ((m.opacity as f64) > LATE_OPAQUE_OPACITY
                && (m.falloff_amount as f64) < LATE_OPAQUE_FALLOFF
                && flags & OPACITY_FLAG == 0))
    {
        class = class & 0xFFFF_C0F0 | 0x100;
    }
    class
}

/// Whether the record casts shadows: refraction mode (loaded flags) always
/// does, else an opacity above 0.8 in double precision and a loaded
/// destination nibble other than ONE (3, ONE_MINUS_SRC_ALPHA, also casts;
/// an alpha-tested word has become ONE/ZERO, destination nibble 0, and casts); NaN casts none. The
/// record's no-shadow flag (extra `0x4`) clears either.
fn casts_shadow(m: &Material) -> bool {
    let flags = exa_render::shader_program::loaded_flags(m);
    let casts = if flags & MODE_MASK == REFRACTION_MODE {
        true
    } else {
        (m.opacity as f64) > SHADOW_OPACITY
            && exa_render::shader_program::loaded_class(m) & 0xF != 1
    };
    casts && m.extra & NO_SHADOWS == 0
}

/// Whether the diffuse colour is pre-scaled by the opacity: every lit
/// record whose blend source nibble is ONE, except in refraction mode. The
/// class is the one after the late rewrite; the mode bits are the type's
/// plug-in-masked ones (`key`), so only a type-1 record is ever in
/// refraction mode here.
fn premultiplies(m: &Material, key: u32) -> bool {
    key & material_flags::LIT != 0
        && key & MODE_MASK != REFRACTION_MODE
        && (blended_class(m) >> 8) & 0xF == 1
}

/// The MatDif rgb before the fall-off floor: zero for an unlit record,
/// otherwise the linear diffuse pre-scaled by the opacity when
/// [`premultiplies`] says so.
fn shaded_diffuse(m: &Material, key: u32) -> Vec3 {
    if key & material_flags::LIT == 0 {
        return Vec3::ZERO;
    }
    let diffuse = material_diffuse(m);
    if premultiplies(m, key) {
        diffuse * m.opacity
    } else {
        diffuse
    }
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
    key: PaintKey,
    shade: Arc<Texels>,
    item_normal: Option<Arc<Texels>>,
    layers: Vec<OwnedLayer>,
    condition: Condition,
    tiling: Tiling,
}

/// A painted texture set's images, for [`ModelLoader::painted_material`].
pub struct PaintedSet {
    key: PaintKey,
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
            .map(|l| PaintLayer {
                mask: &l.mask,
                material: l.material,
                tint: l.tint,
                wear: l.wear,
                library: l.library.iter().map(|t| t.as_deref()).collect(),
                specular: l.specular.iter().map(|t| t.as_deref()).collect(),
                normals: l.normals.iter().map(|t| t.as_deref()).collect(),
            })
            .collect();
        let shade = &self.shade;
        let painted = paint_with(
            shade,
            self.item_normal.as_deref(),
            &layers,
            self.condition,
            self.tiling,
        );
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
        // The painter's surface map is already packed as the original's:
        // intensity, gloss, metal.
        let surface = painted.surface;
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

/// The diffuse colour as the original's material uniform takes it: linear,
/// after legacy materials (type 0) brighten it by 1.5.
fn material_diffuse(m: &Material) -> Vec3 {
    Vec3::from_array(exa_render::material_state::builder_diffuse(m))
}

/// The shading quality the program choice of library materials assumes
/// (the shipped setting; Exarust renders at it).
const SHADING_QUALITY: u32 = 5;

/// The specular colour the material uniform takes: linear, times the
/// record's specular level when the material state keeps its specular bit;
/// the unscaled colour when only a reflection cube asks for it; zero
/// otherwise. A program whose class has bit 0 set (PBR, hair, skin with a
/// scattering colour, translucent) triples it when a specular map is
/// bound.
fn material_specular(m: &Material) -> Vec3 {
    let Some((mat, shd)) = exa_render::shader_program::key_words(m, SHADING_QUALITY) else {
        return Vec3::ZERO;
    };
    let colour = linear_rgb([m.specular[0], m.specular[1], m.specular[2]]);
    let mut specular = if mat & 0x4 != 0 {
        colour * m.specular_level
    } else if mat & 0x10_0000 != 0 {
        colour
    } else {
        return Vec3::ZERO;
    };
    if shd & 1 != 0 && mat & 0x4_0000 != 0 {
        specular *= 3.0;
    }
    specular
}

/// Whether the factory keeps a record's reflection layer (`Mat.TexRef`):
/// the flag bit, which the type's plug-in passes, and the layer.
fn reflects_cube(m: &Material) -> bool {
    exa_render::shader_program::key_words(m, SHADING_QUALITY)
        .is_some_and(|(mat, _)| mat & 0x10_0000 != 0)
}

/// The record's glossiness as stored; type-0 materials below 0.01 are
/// raised to it, as the original's loader does.
fn material_gloss(m: &Material) -> f32 {
    exa_render::material_state::builder_gloss(m)
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

/// An image of a painted map with a box-filtered mip chain. An sRGB map keeps
/// its texels as painted and averages its colour in linear light, as the GPU
/// builds the mips of the original's sRGB paint targets; alpha and linear maps
/// are averaged as stored.
pub(crate) fn painted_image(width: u32, height: u32, rgba: Vec<u8>, encoding: Encoding) -> Image {
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
/// The record's material-mode bits (`0x48`): `0x100` translucent, `0x200`
/// refraction.
const MODE_MASK: u32 = 0x300;
/// The refraction material mode (`0x48`).
const REFRACTION_MODE: u32 = 0x200;
/// A transparent record above this opacity, with a fall-off below
/// [`LATE_OPAQUE_FALLOFF`] and no opacity flag, is rewritten opaque. The
/// original narrows the stored f32 to f64 for the compare.
const LATE_OPAQUE_OPACITY: f64 = 0.99;
/// See [`LATE_OPAQUE_OPACITY`].
const LATE_OPAQUE_FALLOFF: f64 = 0.01;
/// A record above this opacity casts shadows (unless refraction mode or the
/// destination nibble is ONE). The original narrows the stored f32 to f64.
const SHADOW_OPACITY: f64 = 0.8;
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

/// A group's index buffer in the smallest type the original uses: it
/// compares the group's last vertex index with 255 and 65 535, so groups of
/// up to 255 vertices take bytes, up to 65 535 shorts and larger ones
/// 32-bit words. Bevy has no byte indices, so the small groups use u16.
fn group_indices(vertices: usize, local: Vec<u32>) -> Indices {
    if vertices < 0x1_0000 {
        Indices::U16(local.iter().map(|&i| i as u16).collect())
    } else {
        Indices::U32(local)
    }
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
    // The original's splitter: per-vertex tangents from the UV channel and
    // surface normals, in its own handedness convention.
    let tangents = mesh.uvs.as_ref().and_then(|uvs| {
        let uv: Vec<[f32; 2]> = used.iter().map(|&i| uvs[i]).collect();
        crate::tangents::split_tangents(&positions, &normals, &uv, &local)
    });
    // The original picks the smallest index type that fits: bytes up to 255
    // vertices, shorts up to 65 535. Bevy has no u8 indices, so 255 and
    // fewer vertices use u16.
    let indices = group_indices(used.len(), local);
    // Kept in the main world too: static colliders are built from mesh data.
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(indices);
    if let Some(uvs) = &mesh.uvs {
        let uv: Vec<[f32; 2]> = used.iter().map(|&i| uvs[i]).collect();
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }
    // The stored colour stream (`RfcMesh::colors`) is not uploaded: only the
    // dirt modes read it, and Exarust shades none but the blood mode, whose
    // meshes get their own byte stream when a wound lands. Uploading it for
    // every coloured prop would cost 16 bytes a vertex and a pipeline
    // variant for no change in the picture (`exa-tool corpus colours` counts
    // the meshes); the dirt modes 1-3 will need it.
    if let Some(tangents) = tangents {
        m.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
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

/// The frames a scene node has in the body's space: its own transform
/// composed with every ancestor's.
fn scene_frames(scene: &Scene) -> Vec<exa_formats::morph::Frame> {
    let mut out = Vec::with_capacity(scene.nodes.len());
    for index in 0..scene.nodes.len() {
        let mut chain = vec![index];
        let mut current = index;
        while let Some(parent) = scene.nodes[current].parent.and_then(|p| p.checked_sub(1)) {
            if parent >= scene.nodes.len() || chain.contains(&parent) {
                break;
            }
            chain.push(parent);
            current = parent;
        }
        let mut frame = crate::merge::IDENTITY;
        for &at in chain.iter().rev() {
            frame = exa_formats::morph::compose(&frame, &scene.nodes[at].transform);
        }
        out.push(frame);
    }
    out
}

/// The merged `_SKIN` mesh of a scene's skinned nodes, as the original's
/// scene import builds it: every mesh node that has weights contributes its
/// drawn material groups, placed by the node's frame in the body's space.
/// `visible` leaves a node's parts out (a hidden body slot). Returns `None`
/// when no skinned mesh is left.
pub fn scene_merged_mesh(
    scene: &Scene,
    skins: &[Option<(u16, Vec<VertexSkin>)>],
    visible: impl Fn(usize) -> bool,
) -> Option<crate::merge::Merged> {
    use exa_formats::rfc::NodeBody;
    let frames = scene_frames(scene);
    let weights: Vec<Option<(u16, Vec<exa_formats::rfc::SkinWeights>)>> = skins
        .iter()
        .map(|s| {
            s.as_ref().map(|(bone, weights)| {
                let converted = weights
                    .iter()
                    .map(|w| exa_formats::rfc::SkinWeights {
                        bones: w.map(|(b, _)| u32::from(b)),
                        weights: w.map(|(_, x)| x),
                    })
                    .collect();
                (*bone, converted)
            })
        })
        .collect();
    let mut parts = Vec::new();
    for (index, node) in scene.nodes.iter().enumerate() {
        let NodeBody::Mesh(mesh) = &node.body else {
            continue;
        };
        let Some((bone, weights)) = weights.get(index).and_then(Option::as_ref) else {
            continue;
        };
        if !visible(index) {
            continue;
        }
        for group in &mesh.groups {
            if group.material.eq_ignore_ascii_case("_null") || group.index_count == 0 {
                continue;
            }
            let first = group.first_index as usize;
            parts.push(crate::merge::MergePart {
                mesh,
                frame: frames[index],
                skin: Some(weights),
                bone: u32::from(*bone),
                material: &group.material,
                visible: true,
                vertex_masks: None,
                cull: 0,
                occlude: false,
                index_range: Some((first, first + group.index_count as usize)),
            });
        }
    }
    crate::merge::merge(&parts)
}

/// One part of a dressed character to merge: a built material group of a
/// body or garment node, placed by its frame in the body's space.
///
/// The running game has each part as a Bevy [`Mesh`] (one material group of
/// a node's mesh, with its skin attributes); [`merge_dressed`] turns them
/// into [`exa_formats::rfc::Mesh`] streams again and runs
/// [`crate::merge::merge`] over them, the original's assembly.
pub struct DressedPart<'a> {
    /// The part's built mesh.
    pub mesh: &'a Mesh,
    /// Its frame in the body's space.
    pub frame: exa_formats::morph::Frame,
    /// The bone a vertex without a weight falls back to.
    pub bone: u32,
    /// Its material name, as the file stores it.
    pub material: &'a str,
    /// The material its built group draws with.
    pub handle: Handle<ExaMaterial>,
    /// The cull bits the slot's append takes
    /// ([`exa_character::layers::cull_bits`]; 0 for body parts and overlays).
    pub cull: u32,
}

/// A character's merged mesh and the material of each of its runs, in the
/// same order as `Merged::subobjects`.
pub struct DressedMerge {
    pub merged: crate::merge::Merged,
    pub materials: Vec<Handle<ExaMaterial>>,
}

/// Merges a dressed character's built parts into the original's one skinned
/// mesh: each part's vertex streams (positions, normals, UVs, weights) are
/// turned back into the file's form and appended by [`crate::merge::merge`].
/// The handles are matched to the runs by material name; a name shared by
/// two parts (the original groups by name too) uses the first handle met in
/// merge order.
///
/// The position-stream ids are the parts' own vertex numbers (the built
/// meshes keep no stream ids); they only feed the occlusion fade, which no
/// shipped material marks. Triangles' file flags and edge bytes are not
/// carried either, for the same reason.
pub fn merge_dressed(parts: &[DressedPart<'_>]) -> Option<DressedMerge> {
    let streams: Vec<Option<RfcMesh>> =
        parts.iter().map(|part| render_streams(part.mesh)).collect();
    let mut handles: HashMap<String, Handle<ExaMaterial>> = HashMap::new();
    let mut merge_parts = Vec::with_capacity(parts.len());
    for (part, stream) in parts.iter().zip(&streams) {
        let Some(stream) = stream else { continue };
        merge_parts.push(crate::merge::MergePart {
            mesh: stream,
            frame: part.frame,
            skin: stream.skin.as_deref(),
            bone: part.bone,
            material: part.material,
            visible: true,
            vertex_masks: None,
            cull: part.cull,
            occlude: false,
            index_range: None,
        });
    }
    let merged = crate::merge::merge(&merge_parts)?;
    let mut materials = Vec::with_capacity(merged.subobjects.len());
    for run in &merged.subobjects {
        if let Some(handle) = handles.get(run.material.as_str()) {
            materials.push(handle.clone());
            continue;
        }
        let handle = parts
            .iter()
            .zip(&streams)
            .find(|(part, stream)| stream.is_some() && part.material == run.material)
            .map(|(part, _)| part.handle.clone())
            .unwrap_or_default();
        handles.insert(run.material.clone(), handle.clone());
        materials.push(handle);
    }
    Some(DressedMerge { merged, materials })
}

/// A built mesh's streams as the merge takes them: positions, normals, UVs,
/// weights and indices. A mesh without positions or indices has no part.
fn render_streams(mesh: &Mesh) -> Option<RfcMesh> {
    use bevy::mesh::VertexAttributeValues;
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)?
        .as_float3()?
        .to_vec();
    let count = positions.len();
    let indices = match mesh.indices()? {
        Indices::U16(v) => v.iter().map(|&i| u32::from(i)).collect(),
        Indices::U32(v) => v.clone(),
    };
    if count == 0 || indices.is_empty() {
        return None;
    }
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|a| a.as_float3())
        .map(<[[f32; 3]]>::to_vec)
        .filter(|n| n.len() == count)
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; count]);
    let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(VertexAttributeValues::Float32x2(uv)) if uv.len() == count => Some(uv.clone()),
        _ => None,
    };
    let skin = match (
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX),
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
    ) {
        (
            Some(VertexAttributeValues::Uint16x4(joints)),
            Some(VertexAttributeValues::Float32x4(weights)),
        ) if joints.len() == count && weights.len() == count => Some(
            joints
                .iter()
                .zip(weights)
                .map(|(bones, weights)| exa_formats::rfc::SkinWeights {
                    bones: bones.map(u32::from),
                    weights: *weights,
                })
                .collect(),
        ),
        _ => None,
    };
    Some(RfcMesh {
        positions,
        normals,
        uvs,
        colors: None,
        skin,
        position_ids: (0..count as u32).collect(),
        indices,
        groups: Vec::new(),
        triangle_flags: None,
        triangle_edges: None,
        gi_points: Vec::new(),
        render_flags: 0,
        position_count: count,
        smoothing: vec![0; count],
        uv_ids: Vec::new(),
        uv_count: 0,
    })
}

/// One material run of a merged character mesh as a drawable mesh: only the
/// run's vertices and indices, so each run is a sub-mesh of the merged one
/// with the same skin weights. `None` for a run without triangles.
pub fn run_mesh(merged: &crate::merge::Merged, run: &crate::merge::SubObject) -> Option<Mesh> {
    let source = &merged.mesh;
    let first = run.first_vertex.min(source.positions.len());
    let last = (run.first_vertex + run.vertex_count).min(source.positions.len());
    if first >= last {
        return None;
    }
    let indices: Vec<u32> = source
        .indices
        .iter()
        .skip(run.first_index)
        .take(run.index_count)
        .filter(|i| (first..last).contains(&(**i as usize)))
        .map(|i| i - first as u32)
        .collect();
    if indices.is_empty() {
        return None;
    }
    let count = last - first;
    let positions = source.positions[first..last].to_vec();
    let normals = source.normals.get(first..last)?.to_vec();
    // The splitter's tangents, from the run's own triangles as each drawn
    // group's are (`group_mesh`): without them no normal map shades.
    let uvs = source.uvs.as_ref().and_then(|uvs| uvs.get(first..last));
    let tangents =
        uvs.and_then(|uvs| crate::tangents::split_tangents(&positions, &normals, uvs, &indices));
    let max = count as u32;
    let indices = if max < 0x1_0000 {
        Indices::U16(indices.iter().map(|&i| i as u16).collect())
    } else {
        Indices::U32(indices)
    };
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(indices);
    if let Some(uvs) = uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    }
    if let Some(tangents) = tangents {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    }
    if let Some(colours) = source.colors.as_ref().and_then(|c| c.get(first..last)) {
        let bytes: Vec<[u8; 4]> = colours.to_vec();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            bevy::mesh::VertexAttributeValues::Unorm8x4(bytes),
        );
    }
    if let Some(skin) = source.skin.as_ref().and_then(|s| s.get(first..last)) {
        let joints: Vec<[u16; 4]> = skin
            .iter()
            .map(|w| w.bones.map(|b| b.min(u32::from(u16::MAX)) as u16))
            .collect();
        let weights: Vec<[f32; 4]> = skin.iter().map(|w| w.weights).collect();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            bevy::mesh::VertexAttributeValues::Uint16x4(joints),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
        let _ = mesh.generate_skinned_mesh_bounds();
    }
    Some(mesh)
}

/// The `_`-named skinned child of `parent`: the effects' and wound painting's
/// lookup for the copy of a body part they draw on. The original takes the
/// first child whose name starts `_` and that carries a skinned mesh.
pub fn underscore_skinned_child(model: &Model, parent: usize) -> Option<usize> {
    (0..model.nodes.len()).find(|&i| {
        model.nodes[i].parent == Some(parent)
            && model.nodes[i].name.starts_with('_')
            && model.nodes[i].skin_bone.is_some()
    })
}

/// The first skinned mesh in `parent`'s subtree, searched in reverse child
/// order then depth first, as the shell, scan and GUI effects find the mesh
/// they draw on. `ModelNode::skin_bone` stands for the original's skin
/// controller pointer.
pub fn skinned_mesh_under(model: &Model, parent: usize) -> Option<usize> {
    let children: Vec<usize> = (0..model.nodes.len())
        .filter(|&i| model.nodes[i].parent == Some(parent))
        .collect();
    for &child in children.iter().rev() {
        if model.nodes[child].skin_bone.is_some() {
            return Some(child);
        }
        if let Some(found) = skinned_mesh_under(model, child) {
            return Some(found);
        }
    }
    None
}

/// The one texture stem a model's `&` materials are painted from: the first
/// run-time material the original's walk meets, depth first (a node's parts
/// in order, then its children in order, from the roots in order). The walk
/// makes a single job for the model and rewrites every `&` material to that
/// job's stem, so a later part with another stem still takes the first's
/// textures (`docs/formats/OTHER.md`, run-time materials).
pub fn painted_model_stem(model: &Model) -> Option<String> {
    fn walk(model: &Model, node: usize, depth: usize) -> Option<String> {
        let own = model.nodes[node]
            .part_materials
            .iter()
            .filter(|name| name.starts_with('&'))
            .find_map(|name| apparel_texture_key(name));
        if own.is_some() || depth > model.nodes.len() {
            return own;
        }
        (0..model.nodes.len())
            .filter(|&child| child != node && model.nodes[child].parent == Some(node))
            .find_map(|child| walk(model, child, depth + 1))
    }
    (0..model.nodes.len())
        .filter(|&i| model.nodes[i].parent.is_none_or(|p| p >= model.nodes.len()))
        .find_map(|root| walk(model, root, 0))
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
        {
            let (material, appearance, texel, sampled) = (1, condition(160, 64), 96, 0.1170);
            let (tint, wear) = crate::paint::apparel_tint(material, 1);
            let layer = PaintLayer {
                mask: &full,
                material,
                tint,
                wear,
                library: vec![Some(&grey); 4],
                specular: vec![],
                normals: vec![],
            };
            let painted = crate::paint::paint(&shade, None, &[layer], &appearance);
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

    fn material_node(materials: &[&str]) -> ModelNode {
        ModelNode {
            name: "part".into(),
            parent: None,
            transform: Transform::IDENTITY,
            parts: Vec::new(),
            part_materials: materials.iter().map(|m| (*m).to_string()).collect(),
            collision: Vec::new(),
            skin_bone: None,
            light: None,
            voxel_light: None,
            fire: None,
            portal: None,
            bounds: (Vec3::ZERO, 0.0),
            half_extents: Vec3::ZERO,
            gi_points: Arc::new(Vec::new()),
            rigid: None,
            tracks: Vec::new(),
            shadow_bias: 0,
            is_skin: false,
            placeholder: false,
            cloth: false,
            mesh: false,
            render_flags: 0,
        }
    }

    /// One texture job per model: the first `&` material in node order names
    /// the stem every `&` part is painted from, even a later part's own
    /// stem.
    #[test]
    fn a_models_painted_stem_is_the_first_run_time_material() {
        let model = Model {
            nodes: vec![
                material_node(&["skinhuman", "&doublet_ba1"]),
                material_node(&["&otheraaaa1", "&doublet_ba2"]),
            ],
        };
        assert_eq!(painted_model_stem(&model).as_deref(), Some("doublet_ba"));
        // Depth first: a root's child is visited before the next root, even
        // when the next root comes earlier in the node list.
        let mut child = material_node(&["&childstem1"]);
        child.parent = Some(0);
        let model = Model {
            nodes: vec![
                material_node(&["skinhuman"]),
                material_node(&["&secondroo1"]),
                child,
            ],
        };
        assert_eq!(painted_model_stem(&model).as_deref(), Some("childstem1"));
        assert_eq!(
            painted_model_stem(&Model {
                nodes: vec![material_node(&["skinhuman", "&short"])],
            }),
            None
        );
    }

    /// The wound and effect lookups: the `_` child first, and the deepest
    /// skinned mesh in reverse child order.
    #[test]
    fn skinned_meshes_are_found_like_the_effects_do() {
        let mut root = material_node(&[]);
        root.name = "12RUArm".into();
        let mut plain = material_node(&[]);
        plain.name = "plain".into();
        plain.parent = Some(0);
        let mut under = material_node(&[]);
        under.name = "_skin".into();
        under.parent = Some(1);
        under.skin_bone = Some(12);
        let mut underscore = material_node(&[]);
        underscore.name = "_wound".into();
        underscore.parent = Some(0);
        underscore.skin_bone = Some(12);
        let model = Model {
            nodes: vec![root, plain, under, underscore],
        };
        assert_eq!(underscore_skinned_child(&model, 0), Some(3));
        assert_eq!(skinned_mesh_under(&model, 0), Some(3));
        assert_eq!(skinned_mesh_under(&model, 1), Some(2));
        assert_eq!(skinned_mesh_under(&model, 2), None);
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
    fn legacy_materials_scale_before_linearising() {
        // The material uniform brightens a type-0 diffuse colour before the
        // sRGB transfer: 0.5 becomes liner(0.75), not liner(0.5) * 1.5.
        let legacy = Material {
            diffuse: [0.5, 0.25, 0.125, 1.0],
            kind: 0,
            ..default()
        };
        let d = material_diffuse(&legacy);
        assert!((d.x - linear(0.75)).abs() < 1e-6);
        assert!((d.y - linear(0.375)).abs() < 1e-6);
        let pbr = Material {
            kind: 1,
            ..legacy.clone()
        };
        assert!((material_diffuse(&pbr).x - linear(0.5)).abs() < 1e-6);
        // A type-0 glossiness below 0.01 is raised; other kinds keep it.
        let dull = Material {
            glossiness: 0.005,
            ..legacy
        };
        assert_eq!(material_gloss(&dull), 0.01);
        assert_eq!(material_gloss(&Material { kind: 1, ..dull }), 0.005);
    }

    #[test]
    fn specular_needs_the_bit_and_an_odd_class_triples_it() {
        let mut m = Material {
            specular: [1.0, 0.5, 0.25, 1.0],
            specular_level: 0.5,
            kind: 1,
            flags: 0x4 | 0x4_0000,
            ..default()
        };
        let s = material_specular(&m);
        assert!((s.x - 0.5).abs() < 1e-6);
        assert_eq!(
            material_specular(&Material {
                flags: 0,
                ..m.clone()
            }),
            Vec3::ZERO
        );
        m.layers.push(exa_formats::material::TextureLayer {
            slot: TextureSlot::Specular,
            texture: "s".into(),
            parameter: 0,
            amount: 0.0,
        });
        assert!((material_specular(&m).x - 1.5).abs() < 1e-6);
        // Hair (class 3) triples too; legacy (0) and skin without a
        // scattering colour (6) do not.
        let at = |kind| material_specular(&Material { kind, ..m.clone() }).x;
        assert!((at(3) - 1.5).abs() < 1e-6);
        assert!((at(0) - 0.5).abs() < 1e-6);
        assert!((at(4) - 0.5).abs() < 1e-6);
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
        let e = emissive(&glow, 0x0008_0008);
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

    /// The diffuse slot's shading flags: a BC4 upload shows one grey
    /// channel, and the bias-skip word bit turns the shader's bias to zero.
    #[test]
    fn the_diffuse_slot_sets_grey_and_bias_skip_flags() {
        assert_eq!(diffuse_shading_flags(0x700F0, false), 0);
        assert_eq!(
            diffuse_shading_flags(0x700F0, true),
            exa_render::material::flags::GREY_DIFFUSE
        );
        assert_eq!(
            diffuse_shading_flags(0x702F0, false),
            exa_render::material::flags::SKIP_DIFFUSE_BIAS
        );
        assert_eq!(
            diffuse_shading_flags(0x702F0, true),
            exa_render::material::flags::GREY_DIFFUSE
                | exa_render::material::flags::SKIP_DIFFUSE_BIAS
        );
    }

    /// A BC4 image in the specular or emissive slot is one grey channel
    /// too: the shader repeats red over rgb on those paths, as the
    /// hardware's luminance upload did in the original.
    #[test]
    fn the_specular_and_emissive_slots_take_the_grey_flag() {
        use exa_render::material::flags;
        assert_eq!(grey_slot_flag(false, flags::GREY_SPECULAR), 0);
        assert_eq!(
            grey_slot_flag(true, flags::GREY_SPECULAR),
            flags::GREY_SPECULAR
        );
        assert_eq!(
            grey_slot_flag(true, flags::GREY_EMISSIVE),
            flags::GREY_EMISSIVE
        );
        // The three slot bits are distinct and do not overlap the diffuse
        // bias skip.
        assert_eq!(flags::GREY_SPECULAR, 1 << 26);
        assert_eq!(flags::GREY_EMISSIVE, 1 << 27);
        assert_eq!(
            flags::GREY_DIFFUSE | flags::GREY_SPECULAR | flags::GREY_EMISSIVE,
            0x0D00_0000
        );
    }

    #[test]
    fn lit_emissive_maps_double_the_colour() {
        let handle = Material {
            emissive: [1.0, 0.5, 0.0, 1.0],
            flags: 0x0008_000B,
            ..default()
        };
        let e = emissive(&handle, 0x0008_000B);
        assert!(
            (e.truncate() - Vec3::new(2.0, 2.0 * linear(0.5), 0.0))
                .abs()
                .max_element()
                < 1e-5
        );
        assert_eq!(e.w, 1.0);
        // Without the emissive layer or MatEmi the colour stays as it is.
        let plain = Material {
            flags: 0x3,
            ..handle.clone()
        };
        assert_eq!(emissive(&plain, 0x3).x, 1.0);
        let no_mat_emi = Material {
            flags: 0x0008_0003,
            ..handle.clone()
        };
        assert_eq!(emissive(&no_mat_emi, 0x0008_0003).x, 1.0);
    }

    /// A record with one layer per slot, all named after the slot.
    fn with_layers(flags: u32, kind: u32, slots: &[TextureSlot]) -> Material {
        Material {
            flags,
            kind,
            layers: slots
                .iter()
                .map(|&slot| exa_formats::material::TextureLayer {
                    slot,
                    texture: format!("{slot:?}").to_ascii_lowercase(),
                    parameter: 0,
                    amount: 0.0,
                })
                .collect(),
            ..default()
        }
    }

    /// The loader's late rewrite: a destination THREE (the transparent
    /// class) becomes source ONE / destination ZERO in refraction mode, or
    /// with opacity above 0.99, fall-off below 0.01 and no opacity flag.
    /// The comparisons widen the stored f32 to f64, so 0.99f32 and 0.01f32
    /// sit on the passing side of their bounds.
    #[test]
    fn transparent_records_are_rewritten_opaque_late() {
        let mut m = record(0x0203, 0, 1.0);
        assert_eq!(blended_class(&m), 0x100);
        assert_eq!(blend_mode(blended_class(&m), false), AlphaMode::Opaque);
        // 0.99f32 widens above the f64 bound; 0.9899999 is below it.
        m.opacity = 0.99;
        assert_eq!(blended_class(&m), 0x100);
        m.opacity = 0.989_999_9;
        assert_eq!(blended_class(&m), 0x0203);
        // 0.01f32 widens below the f64 bound; 0.010000001 is above it.
        m.opacity = 1.0;
        m.falloff_amount = 0.01;
        assert_eq!(blended_class(&m), 0x100);
        m.falloff_amount = 0.010_000_001;
        assert_eq!(blended_class(&m), 0x0203);
        // The opacity flag keeps the surface blended.
        m.falloff_amount = 0.0;
        m.flags = OPACITY_FLAG;
        assert_eq!(blended_class(&m), 0x0203);
        // So does a loaded opacity layer, which sets the flag at load.
        let mut layered = with_layers(0, 0, &[TextureSlot::Opacity]);
        layered.class = 0x0203;
        layered.opacity = 1.0;
        assert_eq!(blended_class(&layered), 0x0203);
        // The rewrite starts from the alpha-test rewrite's class: an
        // alpha-tested word loses its destination nibble first, so it is
        // not a destination THREE any more.
        assert_eq!(blended_class(&record(0x4203, 0, 1.0)), 0x4100);
        assert_eq!(blended_class(&record(0x4203, REFRACTION_MODE, 1.0)), 0x4100);
        // Refraction rewrites whatever the opacity and fall-off.
        let glass = record(0x0203, REFRACTION_MODE, 0.2);
        assert_eq!(blended_class(&glass), 0x100);
        // Only the destination nibble 3 is rewritten.
        let other = record(0x0204, REFRACTION_MODE, 0.2);
        assert_eq!(blended_class(&other), 0x0204);
        // NaN fails the opacity compare.
        let nan = record(0x0203, 0, f32::NAN);
        assert_eq!(blended_class(&nan), 0x0203);
    }

    /// MatDif.rgb is zero for an unlit record; a lit record with source
    /// nibble ONE premultiplies by the opacity, except in refraction mode.
    #[test]
    fn the_diffuse_colour_is_premultiplied_only_for_a_lit_one_source() {
        let mut m = Material {
            diffuse: [0.5, 0.25, 0.125, 1.0],
            opacity: 0.5,
            class: 0x0103,
            flags: material_flags::LIT,
            ..default()
        };
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(key, material_flags::LIT);
        assert!(premultiplies(&m, key));
        let d = shaded_diffuse(&m, key);
        let linear_half_scale = linear(0.75);
        assert!((d.x - linear_half_scale * 0.5).abs() < 1e-6);
        assert!((d.y - linear(0.375) * 0.5).abs() < 1e-6);
        // Source TWO does not premultiply, whatever the destination.
        m.class = 0x0203;
        assert!(!premultiplies(&m, key));
        assert!((shaded_diffuse(&m, key).x - linear_half_scale).abs() < 1e-6);
        // Source ONE with destination ONE does.
        m.class = 0x0101;
        assert!(premultiplies(&m, key));
        // Refraction mode never premultiplies: it needs a type whose
        // plug-in keeps the mode bits (type 1).
        m.class = 0x0103;
        m.kind = 1;
        m.flags = material_flags::LIT | REFRACTION_MODE;
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(key & MODE_MASK, REFRACTION_MODE);
        assert!(!premultiplies(&m, key));
        // Only type 0 brightens its diffuse by 1.5.
        assert!((shaded_diffuse(&m, key).x - linear(0.5)).abs() < 1e-6);
        // A type whose plug-in drops the mode bits still premultiplies,
        // though the loader rewrote its blend word on the raw flags.
        m.kind = 0;
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(key & MODE_MASK, 0);
        assert!(premultiplies(&m, key));
        m.kind = 1;
        // An unlit record has no diffuse colour at all.
        m.flags = 0;
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(shaded_diffuse(&m, key), Vec3::ZERO);
    }

    /// The emissive colour doubles only when the plug-in-masked word keeps
    /// MatEmi, the lit/specular bits and the emissive layer; an unlit
    /// record's alpha is the stored opacity, unclamped.
    #[test]
    fn emissive_doubling_follows_the_masked_type_flags() {
        let mut m = Material {
            emissive: [1.0, 0.5, 0.0, 1.0],
            opacity: 0.4,
            flags: 0x8 | 0x8_0000 | 0x1,
            ..default()
        };
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(key, 0x8_0009);
        let e = emissive(&m, key);
        assert!((e.x - 2.0).abs() < 1e-6);
        assert!((e.y - 2.0 * linear(0.5)).abs() < 1e-6);
        // Type 2 keeps MatEmi but not the lit bit or the emissive layer.
        m.kind = 2;
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(key, 0x8);
        let e = emissive(&m, key);
        assert!((e.x - 1.0).abs() < 1e-6);
        assert_eq!(e.w, 0.4);
        // Without MatEmi there is no doubling, and alpha is the colour's.
        m.kind = 0;
        m.flags = 0x8_0000 | 0x1;
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(emissive(&m, key).x, 1.0);
        assert_eq!(emissive(&m, key).w, 1.0);
        // The unlit alpha is the opacity as stored, not clamped to 1.
        m.kind = 2;
        m.flags = 0x8;
        m.opacity = 1.5;
        let key = exa_render::shader_program::feature_key(&m);
        assert_eq!(emissive(&m, key).w, 1.5);
    }

    /// Refracting glass casts, a destination ONE never does, and the opacity
    /// compare widens to f64 (so 0.8f32 is above 0.8).
    #[test]
    fn refraction_and_the_destination_nibble_choose_the_shadow_caster() {
        let glass = record(0x0203, REFRACTION_MODE, 0.0);
        assert!(casts_shadow(&glass));
        // 0.8f32 widens above the f64 bound; the next f32 below does not.
        assert!(casts_shadow(&record(0x0203, 0, 0.8)));
        assert!(!casts_shadow(&record(0x0203, 0, 0.799_999_9)));
        assert!(!casts_shadow(&record(0x0201, 0, 1.0)));
        assert!(!casts_shadow(&record(0x0203, 0, f32::NAN)));
        // The loader's alpha-test rewrite clears the destination nibble
        // first, so an alpha-tested destination ONE casts.
        assert!(casts_shadow(&record(0x4201, 0, 1.0)));
        // Refraction wins over destination ONE (read from the loaded flags,
        // whatever the type); extra flag 4 clears either.
        assert!(casts_shadow(&record(0x0201, REFRACTION_MODE, 0.0)));
        assert!(!casts_shadow(&Material {
            extra: NO_SHADOWS,
            ..glass.clone()
        }));
    }

    /// A map binds only where the factory's masked feature word and material
    /// word say so; a type-2 record keeps only the colour/opacity slot.
    #[test]
    fn the_factory_gates_each_texture_slot() {
        use exa_render::shader_program::{feature_key, state_words};
        let maps = |m: &Material| {
            let key = feature_key(m);
            let word = state_words(m, SHADING_QUALITY).map_or(0, |(mat, _)| mat);
            factory_maps(m, key, word)
        };
        // The colour slot binds a diffuse map only for a lit record.
        let mut m = with_layers(0x8, 0, &[TextureSlot::Diffuse]);
        assert!(!maps(&m).diffuse);
        m.flags = 0x1;
        assert!(maps(&m).diffuse);
        // Emissive needs the layer bit and `key & 8`.
        let mut m = with_layers(0x8_0003, 0, &[TextureSlot::Emissive]);
        assert!(!maps(&m).emissive);
        m.flags |= 0x8;
        assert!(maps(&m).emissive);
        // Specular needs the layer bit and `key & 0x100004`.
        let mut m = with_layers(0x4_0003, 0, &[TextureSlot::Specular]);
        assert!(!maps(&m).specular);
        m.flags |= 0x4;
        assert!(maps(&m).specular);
        m.flags = 0x4_0003 | 0x10_0000;
        assert!(maps(&m).specular);
        // Normal needs the layer bit and a shaded material word (`0x100807`).
        let mut m = with_layers(0x20_0003, 0, &[TextureSlot::Normal]);
        assert!(maps(&m).normal);
        m.flags = 0x20_0000;
        m.opacity = 1.0;
        // No low nibble: the factory replaces the whole word with MatEmi.
        assert_eq!(feature_key(&m), 0x8);
        assert!(!maps(&m).normal);
        // Type 2 keeps only MatEmi, two-sided and TexOpc.
        let m = with_layers(
            0x40_0018,
            2,
            &[
                TextureSlot::Diffuse,
                TextureSlot::Opacity,
                TextureSlot::Emissive,
                TextureSlot::Specular,
                TextureSlot::Normal,
                TextureSlot::Gloss,
                TextureSlot::Height,
            ],
        );
        let maps = maps(&m);
        assert!(!maps.diffuse);
        assert!(maps.opacity);
        assert!(!maps.emissive);
        assert!(!maps.specular);
        assert!(!maps.normal);
        assert!(!maps.gloss);
        assert!(!maps.height);
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

    /// Every shipped body and garment builds weights that sum to one
    /// (skipped without the game). The skinner runs on the parsed scenes,
    /// without building meshes.
    #[test]
    fn every_shipped_body_and_garment_skins() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let mut bodies = 0;
        let mut parts = 0;
        for kind in [PackKind::Resource, PackKind::Characters] {
            let Some(pack) = game.pack(kind) else {
                continue;
            };
            for entry in pack.entries() {
                let body = kind == PackKind::Resource && is_body_model(&entry.name);
                if kind != PackKind::Characters && !body {
                    continue;
                }
                let Ok(scene) = Scene::parse(pack.bytes(entry)) else {
                    continue;
                };
                if body {
                    bodies += 1;
                }
                let skins = scene_skins(&scene, true, kind);
                for (index, skin) in skins.into_iter().enumerate() {
                    let Some((bone, skin)) = skin else { continue };
                    let NodeBody::Mesh(mesh) = &scene.nodes[index].body else {
                        continue;
                    };
                    parts += 1;
                    assert_eq!(skin.len(), mesh.positions.len(), "{} {bone}", entry.name);
                    for weights in &skin {
                        let total: f32 = weights.iter().map(|(_, w)| w).sum();
                        assert!(
                            (total - 1.0).abs() < 1e-4,
                            "{} bone {bone}: {weights:?}",
                            entry.name
                        );
                    }
                }
            }
        }
        assert!(
            bodies >= 10 && parts > 2000,
            "{bodies} bodies, {parts} parts"
        );
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

    /// A mask the texture set lacks ends the layer list (skipped without
    /// the game): the braies list four masks, Apparel has two.
    #[test]
    fn a_missing_mask_ends_the_layer_list() {
        use exa_formats::objdb::items;
        use exa_formats::rdb::Database;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let db = Database::parse(
            game.pack(PackKind::Resource)
                .unwrap()
                .get("objdb.rdb")
                .unwrap(),
        )
        .unwrap();
        let braies = items(&db)
            .into_iter()
            .find(|i| i.id == 8)
            .unwrap()
            .appearance
            .unwrap();
        let masks: Vec<u8> = braies.used_layers().map(|l| l.mask).collect();
        assert_eq!(masks, [1, 2, 3, 4]);
        assert!(game.texture("braies__aam3").is_none());
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
        let Paint::Job(job) = loader.paint_job("braies__aa", &braies) else {
            panic!("the braies have a shade map")
        };
        assert_eq!(job.layers.len(), 2);
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

    /// A group's indices use u16 up to 65 535 vertices and u32 beyond. The
    /// original's own cut for bytes is 255 vertices (last index 254).
    #[test]
    fn a_group_picks_the_smallest_index_type() {
        assert!(matches!(group_indices(255, vec![0, 1]), Indices::U16(_)));
        assert!(matches!(group_indices(65_535, vec![0, 1]), Indices::U16(_)));
        assert!(matches!(group_indices(65_536, vec![0, 1]), Indices::U32(_)));
        assert_eq!(group_indices(255, vec![7, 9]), Indices::U16(vec![7, 9]));
    }

    /// The stored colour stream is not uploaded: only the dirt modes read a
    /// vertex colour and Exarust shades none of them from the stream, so the
    /// attribute would cost 16 bytes a vertex for nothing. The group's other
    /// streams are built as before.
    #[test]
    fn a_group_leaves_the_stored_colours_off_the_gpu() {
        let mesh = RfcMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
            colors: Some(vec![[10, 20, 30, 255], [40, 50, 60, 128], [70, 80, 90, 0]]),
            position_ids: vec![0, 1, 2],
            indices: vec![0, 1, 2],
            groups: vec![exa_formats::rfc::MeshGroup {
                material: "mat".into(),
                first_index: 0,
                index_count: 3,
            }],
            triangle_flags: Some(vec![0]),
            triangle_edges: Some(vec![0]),
            smoothing: vec![0; 3],
            uv_ids: vec![0, 1, 2],
            uv_count: 3,
            position_count: 3,
            ..Default::default()
        };
        let built = group_mesh(&mesh, &[2, 0, 1], None).expect("a group");
        assert!(built.attribute(Mesh::ATTRIBUTE_COLOR).is_none());
        assert_eq!(built.count_vertices(), 3);
        assert!(built.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    /// A built part turns back into the merge's streams: its positions,
    /// normals, UVs, weights and indices; two materials become two runs of
    /// the merged mesh, each with its own vertices and its handle, and
    /// `run_mesh` slices them back into drawable sub-meshes.
    #[test]
    fn built_parts_merge_into_one_mesh_and_its_runs() {
        use bevy::mesh::VertexAttributeValues;
        let triangle = |offset: f32, joint: u16| {
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![
                    [offset, 0.0, 0.0],
                    [offset + 1.0, 0.0, 0.0],
                    [offset, 1.0, 0.0],
                ],
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 3])
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_JOINT_INDEX,
                VertexAttributeValues::Uint16x4(vec![[joint, 0, 0, 0]; 3]),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1.0, 0.0, 0.0, 0.0]; 3])
            .with_inserted_indices(Indices::U32(vec![0, 1, 2]))
        };
        let first = triangle(0.0, 3);
        let second = triangle(10.0, 7);
        let first_handle = Handle::<ExaMaterial>::default();
        let second_handle = Handle::<ExaMaterial>::default();
        let parts = [
            DressedPart {
                mesh: &first,
                frame: crate::merge::IDENTITY,
                bone: 3,
                material: "cloth",
                handle: first_handle.clone(),
                cull: 0,
            },
            DressedPart {
                mesh: &second,
                frame: crate::merge::IDENTITY,
                bone: 7,
                material: "leather",
                handle: second_handle.clone(),
                cull: 0,
            },
        ];
        let merged = merge_dressed(&parts).expect("two parts");
        let names: Vec<&str> = merged
            .merged
            .subobjects
            .iter()
            .map(|s| s.material.as_str())
            .collect();
        assert_eq!(names, ["cloth", "leather"]);
        assert_eq!(merged.merged.mesh.positions.len(), 6);
        assert_eq!(merged.merged.mesh.indices, [0, 1, 2, 3, 4, 5]);
        assert_eq!(merged.materials.len(), 2);
        let skin = merged.merged.mesh.skin.as_ref().unwrap();
        assert_eq!(skin[0].bones[0], 3);
        assert_eq!(skin[3].bones[0], 7);
        // Each run is its own compact sub-mesh, its indices rebased.
        for (run, expected) in merged.merged.subobjects.iter().zip([0.0f32, 10.0]) {
            let mesh = run_mesh(&merged.merged, run).expect("a run");
            assert_eq!(mesh.count_vertices(), 3);
            // The run keeps the splitter's tangents, so its normal map shades.
            assert!(mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_some());
            let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
            assert_eq!(positions.as_float3().unwrap()[0], [expected, 0.0, 0.0]);
            assert_eq!(
                mesh.indices().unwrap().iter().collect::<Vec<_>>(),
                [0, 1, 2]
            );
            let joints = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).unwrap();
            let VertexAttributeValues::Uint16x4(joints) = joints else {
                panic!("joints");
            };
            assert_eq!(joints[0][0], if expected == 0.0 { 3 } else { 7 });
        }
    }

    /// The shipped human body merges into one mesh with its material runs
    /// (skipped without the game): every index is in range and every
    /// vertex's weights sum to one.
    #[test]
    fn the_shipped_body_merges_into_one_skin_mesh() {
        use crate::skin::VertexSkin;
        use exa_formats::rfc::Scene;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        let bytes = game
            .pack(crate::PackKind::Resource)
            .and_then(|p| p.get("humanbase.rfc"))
            .unwrap();
        let scene = Scene::parse(bytes).unwrap();
        let skins: Vec<Option<(u16, Vec<VertexSkin>)>> =
            scene_skins(&scene, true, crate::PackKind::Resource);
        let merged = scene_merged_mesh(&scene, &skins, |_| true).expect("a merged body");
        // The eighteen parts' skin and eye groups reference 2 737 of the
        // scene's 7 430 vertices (the `_null` groups' are left out).
        assert_eq!(merged.mesh.positions.len(), 2737);
        assert_eq!(merged.subobjects.len(), 2);
        assert!(!merged.mesh.groups.is_empty());
        assert!(merged.mesh.groups.iter().all(|g| g.material != "_null"));
        let vertices = merged.mesh.positions.len();
        assert!(merged.mesh.indices.iter().all(|&i| (i as usize) < vertices));
        for w in merged.mesh.skin.as_ref().unwrap() {
            assert!((w.weights.iter().sum::<f32>() - 1.0).abs() < 1e-4);
        }
    }
}
