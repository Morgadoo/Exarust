//! A character's physique on its body and on the apparel fitted to it, as
//! the original's character builder morphs them (`exa_formats::morph`).
//!
//! A race with a morph body (`<stem>morph.rfc`: humans and ancients) gives
//! every body part and every garment piece its share of the morphs once
//! (the transfer, cached per race and model), and each character a variant
//! of its body and of each garment at its record's weights: the meshes'
//! positions and normals morphed, the body parts' bounds refitted and
//! their translations moved with their joints, so the skeleton, the skin
//! and the physical body built from them follow the physique. A garment
//! piece is carried over from the frame of the body part its node names,
//! as the original places a garment's pieces on its base body before
//! fitting them. The body's parts are skinned on their morphed shape and
//! joints, as the original skins the dressed character; a garment takes
//! its weights from the unmorphed body, before it is morphed. Weights that
//! change nothing (every target below the smallest weight) leave the shared
//! model in place.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use bevy::prelude::*;
use exa_formats::morph::{self, Frame, MorphBody, PartMorph, Physique};
use exa_formats::rfc::{NodeBody, Scene};
use exa_formats::rsq::bone_index;
use exa_rpk::PackKind;

use crate::model::{Model, ModelJob, ModelLoader, is_body_model};

/// Body parts a morph body has landmarks for (bones 1–18).
const PARTS: u16 = 18;

/// What a character's body is morphed by: its race's body model stem
/// (`human`, `ancient`) and its record's physique. On a body model root;
/// the apparel it is dressed in is morphed the same way.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BodyMorph {
    pub stem: String,
    pub physique: Physique,
}

impl BodyMorph {
    /// The morph of a body of `model` (`humanbase.rfc` → stem `human`).
    pub fn of_model(model: &str, physique: Physique) -> Self {
        let lower = model.to_ascii_lowercase();
        let stem = lower.strip_suffix("base.rfc").unwrap_or(&lower).to_string();
        Self { stem, physique }
    }

    /// The cache key of the variants it makes: the stem and the physique's
    /// bits.
    pub fn key(&self) -> String {
        let p = &self.physique;
        let mut key = self.stem.clone();
        for v in [p.build[0], p.build[1], p.height, p.age, p.feral] {
            let _ = write!(key, ":{:08x}", v.to_bits());
        }
        key.push_str(if p.female { ":f" } else { ":m" });
        key
    }
}

/// A physique as a debug flag writes it: the two physique values, then
/// optionally the height, age and feral level (0 when left out) and the
/// female bit (any value but 0), comma-separated (`0,1,-1` is tall and
/// slight, `1,1,1` short and broad).
pub fn parse_physique(text: &str) -> Option<Physique> {
    let v: Vec<f32> = text
        .split(',')
        .map(|x| x.trim().parse().ok())
        .collect::<Option<_>>()?;
    if !(2..=6).contains(&v.len()) || v.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let at = |i: usize| v.get(i).copied().unwrap_or(0.0);
    Some(Physique {
        build: [v[0], v[1]],
        height: at(2).clamp(-1.0, 1.0),
        age: at(3).clamp(0.0, 1.0),
        feral: at(4).clamp(0.0, 1.0),
        female: at(5) != 0.0,
    })
}

/// Morph bodies, base frames and transferred shares, once loaded.
#[derive(Default)]
pub(crate) struct MorphCache {
    /// Morph bodies by race stem (`None` when the race has none).
    bodies: HashMap<String, Option<Arc<MorphBody>>>,
    /// The race's base body parts' frames in the model's space, by bone.
    frames: HashMap<String, Arc<HashMap<u16, Frame>>>,
    /// Each model node's share of the morphs, by race stem and model.
    pub(crate) parts: HashMap<String, Arc<Vec<Option<PartMorph>>>>,
}

/// What morphing a model at a character's weights takes, owned so that the
/// model can be built on any thread.
pub(crate) struct JobMorph {
    /// The variant's cache key (the model's, then the morph's).
    pub(crate) key: String,
    pub(crate) weights: Vec<f32>,
    body: Arc<MorphBody>,
    frames: Arc<HashMap<u16, Frame>>,
    pub(crate) parts_key: String,
    parts: Option<Arc<Vec<Option<PartMorph>>>>,
}

/// A built variant's key and the shares it used, for the cache.
pub(crate) struct BuiltMorph {
    pub(crate) key: String,
    pub(crate) parts_key: String,
    pub(crate) parts: Arc<Vec<Option<PartMorph>>>,
}

impl JobMorph {
    /// Each node's share of the morphs: cached, or transferred now. A body
    /// part is carried over from its own frame in the body model, a garment
    /// piece from the frame of the body part its node names; nodes that name
    /// no body part take none.
    pub(crate) fn parts(&self, scene: &Scene, body_model: bool) -> Arc<Vec<Option<PartMorph>>> {
        if let Some(parts) = &self.parts {
            return parts.clone();
        }
        let own = morph::model_frames(scene);
        Arc::new(
            scene
                .nodes
                .iter()
                .enumerate()
                .map(|(i, node)| {
                    let NodeBody::Mesh(mesh) = &node.body else {
                        return None;
                    };
                    let bone =
                        bone_index(&node.name).filter(|b| (1..=PARTS as usize).contains(b))?;
                    let frame = if body_model {
                        own[i]
                    } else {
                        *self.frames.get(&(bone as u16))?
                    };
                    Some(self.body.transfer(mesh, &frame))
                })
                .collect(),
        )
    }

    /// Morphs `scene` with `parts` at the weights.
    pub(crate) fn apply(&self, scene: &mut Scene, parts: &[Option<PartMorph>]) {
        morph::morph_scene(scene, parts, &self.weights);
    }

    pub(crate) fn built(self, parts: Arc<Vec<Option<PartMorph>>>) -> BuiltMorph {
        BuiltMorph {
            key: self.key,
            parts_key: self.parts_key,
            parts,
        }
    }
}

/// The cache key of a model of `pack`.
pub(crate) fn model_key(pack: PackKind, entry: &str) -> String {
    format!("{}/{}", pack.file_stem(), entry.to_ascii_lowercase())
}

impl ModelLoader<'_> {
    /// The morph body of race stem `stem`: the first node of
    /// `<stem>morph.rfc` carrying a morph set, placed by its frame.
    pub fn morph_body(&mut self, stem: &str) -> Option<Arc<MorphBody>> {
        let stem = stem.to_ascii_lowercase();
        if let Some(body) = self.cache.morph.bodies.get(&stem) {
            return body.clone();
        }
        let body = self
            .game
            .pack(PackKind::Resource)
            .and_then(|p| p.get(&format!("{stem}morph.rfc")))
            .and_then(|b| Scene::parse(b).ok())
            .and_then(|s| MorphBody::of_scene(&s))
            .map(Arc::new);
        self.cache.morph.bodies.insert(stem, body.clone());
        body
    }

    /// The race's base body parts' frames in the model's space, by bone.
    fn bone_frames(&mut self, stem: &str) -> Arc<HashMap<u16, Frame>> {
        if let Some(frames) = self.cache.morph.frames.get(stem) {
            return frames.clone();
        }
        let frames: HashMap<u16, Frame> = self
            .game
            .pack(PackKind::Resource)
            .and_then(|p| p.get(&format!("{stem}base.rfc")))
            .and_then(|b| Scene::parse(b).ok())
            .map(|scene| {
                let frames = morph::model_frames(&scene);
                scene
                    .nodes
                    .iter()
                    .zip(frames)
                    .filter_map(|(n, f)| Some((bone_index(&n.name)? as u16, f)))
                    .collect()
            })
            .unwrap_or_default();
        let frames = Arc::new(frames);
        self.cache.morph.frames.insert(stem.into(), frames.clone());
        frames
    }

    /// What morphing `entry` of `pack` by `morph` takes; `None` when it
    /// leaves the model as it is (no morph, a race without a morph body,
    /// weights below the smallest).
    pub(crate) fn job_morph(
        &mut self,
        pack: PackKind,
        entry: &str,
        morph: Option<&BodyMorph>,
    ) -> Option<JobMorph> {
        let morph = morph?;
        let body = self.morph_body(&morph.stem)?;
        let weights = morph.physique.weights(body.set.targets.len());
        if !morph::morphs(&weights) {
            return None;
        }
        let frames = self.bone_frames(&morph.stem);
        let parts_key = format!("{}#{}", morph.stem, model_key(pack, entry));
        let parts = self.cache.morph.parts.get(&parts_key).cloned();
        Some(JobMorph {
            key: format!("{}#{}", model_key(pack, entry), morph.key()),
            weights,
            body,
            frames,
            parts_key,
            parts,
        })
    }

    /// The cache key `entry` of `pack` has when morphed by `morph`: its own
    /// when the morph leaves it as it is.
    pub fn morphed_key(
        &mut self,
        pack: PackKind,
        entry: &str,
        morph: Option<&BodyMorph>,
    ) -> String {
        match self.job_morph(pack, entry, morph) {
            Some(m) => m.key,
            None => model_key(pack, entry),
        }
    }

    /// `entry` of `pack` morphed by `morph` (the shared model when the morph
    /// leaves it as it is), built now if it is not cached.
    pub fn morphed_model(
        &mut self,
        pack: PackKind,
        entry: &str,
        morph: Option<&BodyMorph>,
    ) -> Option<Arc<Model>> {
        let key = self.morphed_key(pack, entry, morph);
        if let Some(cached) = self.cached_model(&key) {
            return cached;
        }
        let started = std::time::Instant::now();
        let Some(job) = self.morphed_job(pack, entry, morph) else {
            // Missing or unreadable: remembered as such.
            self.cache_model(key, None);
            return None;
        };
        let built = job.build(bevy::tasks::ComputeTaskPool::try_get());
        debug!("model {key} built in {:?}", started.elapsed());
        self.finish_model(built)
    }

    /// The variant of `entry` of `pack` morphed by `morph` when it has been
    /// built (`Some(None)` when it could not be).
    pub fn loaded_morphed_model(
        &mut self,
        pack: PackKind,
        entry: &str,
        morph: Option<&BodyMorph>,
    ) -> Option<Option<Arc<Model>>> {
        let key = self.morphed_key(pack, entry, morph);
        self.cached_model(&key)
    }

    /// [`ModelLoader::model_job`] for the variant morphed by `morph`. The
    /// unmorphed model is loaded first: apparel takes its weights from the
    /// unmorphed body.
    pub fn morphed_job(
        &mut self,
        pack: PackKind,
        entry: &str,
        morph: Option<&BodyMorph>,
    ) -> Option<ModelJob> {
        let job_morph = self.job_morph(pack, entry, morph);
        if job_morph.is_some() && pack == PackKind::Resource && is_body_model(entry) {
            self.model(pack, entry);
        }
        let mut job = self.model_job(pack, entry)?;
        job.morph = job_morph;
        Some(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_morphs_key_names_its_race_and_physique() {
        let m = BodyMorph::of_model("HumanBase.rfc", Physique::NEUTRAL);
        assert_eq!(m.stem, "human");
        assert_eq!(
            m.key(),
            "human:3f000000:3f800000:00000000:00000000:00000000:m"
        );
        let other = BodyMorph {
            physique: Physique {
                female: true,
                ..Physique::NEUTRAL
            },
            ..m.clone()
        };
        assert_ne!(m.key(), other.key());
        assert_eq!(
            BodyMorph::of_model("ancientbase.rfc", Physique::NEUTRAL).stem,
            "ancient"
        );
    }

    /// The human body and a garment morphed by a short, broad physique
    /// (skipped without the game): the neutral physique keeps the shared
    /// model; the variant is cached; the pelvis comes down 11.2 cm with the
    /// height and the joints with it; the garment's vertices move, its skin
    /// weights stay the unmorphed ones.
    #[test]
    fn shipped_bodies_take_the_physique() {
        use crate::model::{BODY_MODEL, ModelCache};
        use exa_render::material::ExaMaterial;
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
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
        let base = loader.model(PackKind::Resource, BODY_MODEL).unwrap();
        let neutral = BodyMorph::of_model(BODY_MODEL, Physique::NEUTRAL);
        let same = loader
            .morphed_model(PackKind::Resource, BODY_MODEL, Some(&neutral))
            .unwrap();
        assert!(Arc::ptr_eq(&base, &same));
        let short = BodyMorph::of_model(
            BODY_MODEL,
            Physique {
                build: [1.0, 1.0],
                height: 1.0,
                ..Physique::NEUTRAL
            },
        );
        let morphed = loader
            .morphed_model(PackKind::Resource, BODY_MODEL, Some(&short))
            .unwrap();
        let again = loader
            .morphed_model(PackKind::Resource, BODY_MODEL, Some(&short))
            .unwrap();
        assert!(Arc::ptr_eq(&morphed, &again));
        assert!(!Arc::ptr_eq(&base, &morphed));
        let node = |m: &Model, name: &str| m.nodes.iter().find(|n| n.name == name).unwrap().clone();
        let (p0, p1) = (node(&base, "01Pelvis"), node(&morphed, "01Pelvis"));
        let drop = (p1.transform.translation.y - p0.transform.translation.y) * 100.0;
        assert!((drop + 11.20).abs() < 0.02, "{drop}");
        // The knee comes 5.7 cm closer to the hip (11.92 − 6.21).
        let (s0, s1) = (node(&base, "03RLLeg"), node(&morphed, "03RLLeg"));
        let closer =
            (s0.transform.translation.length() - s1.transform.translation.length()) * 100.0;
        assert!((closer - 5.7).abs() < 0.3, "{closer}");
        // The parts are skinned and their boxes refitted.
        assert_eq!(p1.skin_bone, p0.skin_bone);
        assert_ne!(p1.half_extents, p0.half_extents);
        let positions = |m: &Handle<Mesh>| {
            loader
                .meshes
                .get(m)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap()
                .to_vec()
        };
        let (a, b) = (positions(&p0.parts[0].0), positions(&p1.parts[0].0));
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).any(|(x, y)| x != y));
        // A garment follows: positions move, weights stay.
        let entry = "trouser_a";
        let plain = loader
            .morphed_model(PackKind::Characters, entry, None)
            .unwrap();
        let fitted = loader
            .morphed_model(PackKind::Characters, entry, Some(&short))
            .unwrap();
        let part = |m: &Model| {
            m.nodes
                .iter()
                .find(|n| n.skin_bone.is_some() && !n.parts.is_empty())
                .unwrap()
                .parts[0]
                .0
                .clone()
        };
        let (h0, h1) = (part(&plain), part(&fitted));
        let (m0, m1) = (
            loader.meshes.get(&h0).unwrap(),
            loader.meshes.get(&h1).unwrap(),
        );
        let p = |m: &Mesh| {
            m.attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap()
                .to_vec()
        };
        let moved = p(m0).iter().zip(p(m1)).filter(|(x, y)| *x != y).count();
        assert!(moved > 0);
        assert_eq!(
            m0.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).map(|a| a.len()),
            m1.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).map(|a| a.len())
        );
        assert_eq!(
            format!("{:?}", m0.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)),
            format!("{:?}", m1.attribute(Mesh::ATTRIBUTE_JOINT_INDEX))
        );
        // Races without a morph body keep their model.
        let skeleton = BodyMorph::of_model("skelbase.rfc", short.physique);
        let bones = loader.model(PackKind::Resource, "skelbase.rfc").unwrap();
        let kept = loader
            .morphed_model(PackKind::Resource, "skelbase.rfc", Some(&skeleton))
            .unwrap();
        assert!(Arc::ptr_eq(&bones, &kept));
    }

    #[test]
    fn physiques_from_the_command_line() {
        assert_eq!(parse_physique("0.5,1"), Some(Physique::NEUTRAL));
        let p = parse_physique("1, 1, 1, 0.5, 2, 1").unwrap();
        assert_eq!(
            (p.build, p.height, p.age, p.feral, p.female),
            ([1.0, 1.0], 1.0, 0.5, 1.0, true)
        );
        assert_eq!(parse_physique("0,1,-3").unwrap().height, -1.0);
        for bad in ["", "1", "a,b", "1,2,3,4,5,6,7", "1,inf"] {
            assert_eq!(parse_physique(bad), None, "{bad}");
        }
    }
}
