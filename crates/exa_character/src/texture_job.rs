//! The skin texture job: what the generator asks the renderer to build for a
//! character, as data. The original allocates a job object, sets its side
//! and format from the pixel-format setting, fills its colour, gloss, beard
//! and normal-map layers, draws two UV offsets from the record's look seed,
//! and hands it to the texture manager; the renderer's passes are the only
//! part Exarust cannot run in a test.
//!
//! This module reproduces the job's fields and the start/first-apparel/reset
//! passes. A renderer can execute it as-is; `exa_assets::human_skin` draws
//! the parts it already understands.

use exa_formats::characters::CharacterRecord;

use crate::factory::SkinFormat;
use crate::look::{
    NormalLayer, SkinSet, beard_index, beard_textures, look_seed, normal_layers, skin_darkening,
    skin_set, skin_textures, skin_uv_offset,
};
use exa_formats::races::LOOK_FEMALE_TEXTURES;

/// The generator's skin job.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinJob {
    /// The body model's stem (`human`), which names the race and its skin
    /// kind in the composite job.
    pub stem: String,
    pub format: SkinFormat,
    /// The race's colour and gloss maps, when it has a skin set.
    pub colour: Option<String>,
    pub gloss: Option<String>,
    /// The race's six normal maps, by physique index.
    pub normals: [Option<String>; 6],
    /// The record's two tone words.
    pub tone: [u16; 2],
    /// The look seed the offsets were drawn from: the record's own, or the
    /// fresh one that replaced a zero, which the caller writes back to the
    /// record as the original does.
    pub seed: u32,
    /// The two skin-map offsets drawn from the look seed.
    pub uv_offset: [f32; 2],
    /// The feral level the skin darkens by, when it does.
    pub darkening: Option<f32>,
    /// The beard's colour and normal maps.
    pub beard: Option<[String; 2]>,
    /// The beard's colour: the hair picker's diffuse for the record's
    /// colour bytes (without the feral level), halfway to grey 0.3.
    pub beard_colour: Option<[f32; 3]>,
    /// The normal-map blend layers.
    pub layers: Vec<NormalLayer>,
    /// The first apparel texture the job carries.
    pub first_apparel: Option<String>,
    /// Whether the job has been handed to the texture manager.
    pub started: bool,
}

impl SkinJob {
    /// A new job at the current pixel-format setting: the side and format
    /// words, no maps, not started.
    pub fn new(pixel_format: u32) -> Self {
        Self {
            stem: String::new(),
            format: SkinFormat::of_setting(pixel_format),
            colour: None,
            gloss: None,
            normals: std::array::from_fn(|_| None),
            tone: [0; 2],
            seed: 0,
            uv_offset: [0.0; 2],
            darkening: None,
            beard: None,
            beard_colour: None,
            layers: Vec::new(),
            first_apparel: None,
            started: false,
        }
    }

    /// Fills the job's look from a record and race: the tone words, the
    /// look seed (a zero seed replaced by `fresh`) and its two offsets, the
    /// darkening, and the beard with its colour. `exists` says whether a
    /// beard texture is shipped.
    pub fn with_look(
        mut self,
        record: &CharacterRecord,
        look_word: u32,
        fresh: &mut dyn FnMut() -> u32,
        exists: &dyn Fn(&str) -> bool,
    ) -> Self {
        self.tone = record.skin;
        self.seed = look_seed(record.search_seed, fresh);
        self.uv_offset = skin_uv_offset(self.seed);
        self.darkening = skin_darkening(record, look_word);
        self.beard = beard_index(record, look_word)
            .and_then(beard_textures)
            .filter(|b| exists(&b[0]) && exists(&b[1]));
        let [h, s, v, _] = record.hair_colour;
        self.beard_colour = self.beard.is_some().then(|| beard_colour([h, s, v]));
        self
    }

    /// Fills the race texture names for a body stem.
    pub fn with_stem(mut self, stem: &str, set: SkinSet) -> Self {
        self.stem = stem.to_ascii_lowercase();
        if let Some(textures) = skin_textures(stem, set) {
            self.colour = Some(textures.colour);
            self.gloss = Some(textures.gloss);
            self.normals = std::array::from_fn(|i| Some(textures.normals[i].clone()));
        }
        self
    }

    /// The job's normal names (for the layer builder).
    fn normal_names(&self) -> [String; 6] {
        std::array::from_fn(|i| self.normals[i].clone().unwrap_or_default())
    }

    /// The first-apparel pass: the job takes the first
    /// texture of a mesh part that has one, scanning in node order.
    pub fn take_first_apparel(&mut self, candidates: &[Option<String>]) -> Option<String> {
        let first = candidates.iter().flatten().next()?.clone();
        self.first_apparel = Some(first.clone());
        self.started = true;
        Some(first)
    }

    /// Resets the job for a new texture pass: the side and format are read
    /// again from the current pixel-format setting, and the first apparel
    /// texture and the started flag are cleared. The maps, tone, beard and
    /// layers stay until the next look fills them.
    pub fn reset(&mut self, pixel_format: u32) {
        self.format = SkinFormat::of_setting(pixel_format);
        self.first_apparel = None;
        self.started = false;
    }

    /// The job with a look, in one call: the stem's textures, the record's
    /// tone, seed, darkening and beard, and the layers whose maps ship.
    /// `controller_weights` are the morph controller's normal weights (empty
    /// for a body without one).
    ///
    /// The layers index the race's maps by sex: a woman of a race that
    /// ships only the men's set (look bit 0 clear) draws the men's colour,
    /// gloss and base normal map but has no layer maps, so she gets none.
    pub fn of_record(
        pixel_format: u32,
        stem: &str,
        record: &CharacterRecord,
        look_word: u32,
        controller_weights: &[f32],
        fresh: &mut dyn FnMut() -> u32,
        exists: &dyn Fn(&str) -> bool,
    ) -> Self {
        let set = skin_set(look_word, record.female);
        let mut job = Self::new(pixel_format)
            .with_stem(stem, set)
            .with_look(record, look_word, fresh, exists);
        let names = if record.female && look_word & LOOK_FEMALE_TEXTURES == 0 {
            Default::default()
        } else {
            job.normal_names()
        };
        let has_extra = !names[5].is_empty();
        job.layers = normal_layers(
            controller_weights,
            usize::from(record.female),
            record.feral,
            &names,
            has_extra,
        )
        .into_iter()
        .filter(|l| exists(&l.texture))
        .collect();
        job
    }
}

/// One draw of the texture job's render callback, in the order the original
/// issues it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinPass {
    /// `chargen_d` over the colour target.
    Colour,
    /// The beard's colour overlay, when the record wears one.
    BeardColour,
    /// `chargen_n` over the normal target.
    Normal,
    /// The beard's normal overlay.
    BeardNormal,
    /// `chargen_s` over the surface target.
    Specular,
    /// The beard's surface overlay.
    BeardSurface,
}

/// The passes a job's callback draws: the colour target, then the normal
/// target, then the surface target, each skin program followed by its beard
/// overlay when the record has a beard. The job's apparel layers are drawn
/// into each target after its beard overlay.
pub fn passes(beard: bool) -> Vec<SkinPass> {
    let mut out = vec![SkinPass::Colour];
    if beard {
        out.push(SkinPass::BeardColour);
    }
    out.push(SkinPass::Normal);
    if beard {
        out.push(SkinPass::BeardNormal);
    }
    out.push(SkinPass::Specular);
    if beard {
        out.push(SkinPass::BeardSurface);
    }
    out
}

impl SkinJob {
    /// The passes the job's callback draws (see [`passes`]).
    pub fn passes(&self) -> Vec<SkinPass> {
        passes(self.beard.is_some())
    }

    /// The job as the loader's CPU composite takes it: the race's colour,
    /// gloss and base normal maps, the tone and its seed's offsets, the
    /// beard and its tint, and the normal layers. `None` without a race
    /// skin set (the body then keeps its own skin material).
    pub fn composite_job(&self) -> Option<exa_assets::SkinCompositeJob> {
        let colour = self.colour.clone()?;
        let gloss = self.gloss.clone()?;
        let base_normal = self.normals[0].clone()?;
        Some(exa_assets::SkinCompositeJob {
            stem: self.stem.clone(),
            tone: self.tone,
            seed: self.seed,
            uv_offset: self.uv_offset,
            colour: Some(colour),
            gloss: Some(gloss),
            base_normal: Some(base_normal),
            layers: self
                .layers
                .iter()
                .map(|l| (l.amount, l.texture.clone()))
                .collect(),
            beard: self.beard.as_ref().and_then(|beard| {
                Some(exa_assets::SkinBeard {
                    colour: beard[0].clone(),
                    normal: beard[1].clone(),
                    tint: self.beard_colour?,
                })
            }),
            side: self.format.side,
        })
    }
}

/// The beard's colour: the hair picker on the record's three colour bytes
/// (each `byte × 1/255` in double), its diffuse moved halfway to grey 0.3.
pub fn beard_colour(colour: [u8; 3]) -> [f32; 3] {
    let unit = |b: u8| (f64::from(b) * (1.0 / 255.0)) as f32;
    let (diffuse, _) = exa_assets::hair::pick(unit(colour[0]), unit(colour[1]), unit(colour[2]));
    diffuse.map(|c| c + (0.3 - c) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The callback's draw order: colour, normal, surface, each skin
    /// program before its beard overlay, and no beard pass without a beard.
    #[test]
    fn the_passes_follow_the_callback_order() {
        assert_eq!(
            passes(false),
            [SkinPass::Colour, SkinPass::Normal, SkinPass::Specular]
        );
        assert_eq!(
            passes(true),
            [
                SkinPass::Colour,
                SkinPass::BeardColour,
                SkinPass::Normal,
                SkinPass::BeardNormal,
                SkinPass::Specular,
                SkinPass::BeardSurface
            ]
        );
    }

    #[test]
    fn the_job_starts_at_the_pixel_formats_side() {
        let job = SkinJob::new(2);
        assert_eq!((job.format.side, job.format.format), (0x200, 0x200));
        assert!(!job.started);
        assert!(job.normals.iter().all(Option::is_none));
        assert_eq!(SkinJob::new(3).format.side, 0x400);
    }

    #[test]
    fn the_first_apparel_texture_wins() {
        let mut job = SkinJob::new(2);
        let candidates = [
            None,
            Some("&doublet_ba1".to_string()),
            Some("&otheraaaa1".to_string()),
        ];
        assert_eq!(
            job.take_first_apparel(&candidates).as_deref(),
            Some("&doublet_ba1")
        );
        assert!(job.started);
        assert_eq!(job.first_apparel.as_deref(), Some("&doublet_ba1"));
        let mut empty = SkinJob::new(2);
        assert_eq!(empty.take_first_apparel(&[None, None]), None);
        assert!(!empty.started);
    }

    #[test]
    fn a_reset_clears_the_maps_and_the_started_flag() {
        let mut job = SkinJob::new(2);
        job.take_first_apparel(&[Some("&x".into())]);
        job.colour = Some("humandifm".into());
        job.beard = beard_textures(0);
        job.reset(3);
        // The maps stay; the format is read again and the pass restarts.
        assert!(job.colour.is_some() && job.beard.is_some());
        assert!(!job.started && job.first_apparel.is_none());
        assert_eq!(job.format, SkinFormat::of_setting(3));
    }

    /// A human woman's job: the women's maps, the tone, the seed's offsets
    /// and the beard absent; the normal layers choose the women's indices.
    #[test]
    fn a_human_womans_job() {
        let mut record = CharacterRecord {
            female: true,
            skin: [0x8000, 0x8000],
            search_seed: 1,
            ..Default::default()
        };
        record.feral = 0.0;
        let exists = |_: &str| true;
        let job = SkinJob::of_record(3, "human", &record, 0x1BF, &[0.0; 12], &mut || 9, &exists);
        assert_eq!(job.format.side, 0x400);
        assert_eq!(job.colour.as_deref(), Some("humandiff"));
        assert_eq!(job.gloss.as_deref(), Some("humanspcf"));
        // Both sexes take the men's extra map.
        assert_eq!(job.normals[5].as_deref(), Some("humannrmm5"));
        assert_eq!(job.tone, [0x8000, 0x8000]);
        assert_eq!((job.seed, job.uv_offset), (1, skin_uv_offset(1)));
        assert!(job.beard.is_none() && job.beard_colour.is_none());
        assert!(job.layers.is_empty());
        // A zero seed is replaced by a fresh one.
        record.search_seed = 0;
        let job = SkinJob::of_record(3, "human", &record, 0x1BF, &[], &mut || 9, &exists);
        assert_eq!((job.seed, job.uv_offset), (9, skin_uv_offset(9)));
    }

    /// A woman of a race with only the men's set (look bits 1 and 7, no
    /// bit 0) draws the men's maps but gets no layers: her race's layer
    /// maps are never loaded.
    #[test]
    fn a_woman_of_a_men_only_race_gets_no_layers() {
        let record = CharacterRecord {
            female: true,
            search_seed: 1,
            ..Default::default()
        };
        let mut weights = [0.0f32; 12];
        weights[6] = 0.5;
        weights[4] = 0.5;
        let exists = |_: &str| true;
        let job = SkinJob::of_record(2, "ancient", &record, 0x86, &weights, &mut || 1, &exists);
        assert_eq!(job.colour.as_deref(), Some("ancientdifm"));
        assert!(job.layers.is_empty());
        // A man of that race blends his.
        let man = CharacterRecord {
            search_seed: 1,
            ..Default::default()
        };
        weights[0] = 0.5;
        let job = SkinJob::of_record(2, "ancient", &man, 0x86, &weights, &mut || 1, &exists);
        let textures: Vec<&str> = job.layers.iter().map(|l| l.texture.as_str()).collect();
        assert_eq!(textures, ["ancientnrmm1", "ancientnrmm5"]);
    }

    /// A character of `exanima01` composites its skin from the race's maps,
    /// its tone, look seed and layers: the material gets the three generated
    /// maps, and the colour map is not empty or uniform (skipped without the
    /// game).
    #[test]
    fn a_shipped_characters_skin_composites() {
        use exa_assets::{ModelCache, ModelLoader, PackKind};
        use exa_formats::morph::Physique;

        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let resource = game.pack(PackKind::Resource).unwrap();
        let scene = exa_formats::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let items = exa_formats::objdb::items(&db);
        let locale =
            exa_formats::roles::locale_id(resource.get("locales.rdb").unwrap(), "exanima01.rfc");
        let characters = exa_world::placed_characters(&scene, &items, locale);
        let Some(character) = characters.iter().find(|c| c.name == "Kade") else {
            return;
        };
        let races = exa_formats::races::parse(resource.get("races.rdb").unwrap()).unwrap();
        let look = exa_formats::races::of_character(&races, character.race).map_or(0, |r| r.look);
        let physique = Physique::of_record(&character.record);
        let weights = physique.weights(12);
        let exists = |name: &str| game.texture(name).is_some();
        let mut job = SkinJob::of_record(
            crate::factory::PIXEL_FORMAT,
            "human",
            &character.record,
            look,
            &weights,
            &mut || 1,
            &exists,
        );
        // A small side keeps the test quick; the maps are the same program.
        job.format.side = 64;
        job.format.format = 64;
        let composite = job.composite_job().expect("the human skin set ships");
        assert_eq!(composite.stem, "human");
        assert_eq!(composite.colour.as_deref(), Some("humandifm"));
        assert_eq!(composite.side, 64);
        let mut cache = ModelCache::default();
        let mut meshes = bevy::prelude::Assets::<bevy::prelude::Mesh>::default();
        let mut materials = bevy::prelude::Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = bevy::prelude::Assets::<bevy::prelude::Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let handle = loader.composited_skin(&composite).unwrap();
        // The second request is the cached material.
        assert_eq!(loader.composited_skin(&composite), Some(handle.clone()));
        let material = loader.materials.get(&handle).unwrap();
        let colour = material.diffuse_map.clone().unwrap();
        let surface = material.specular_map.clone().unwrap();
        let normal = material.normal_map.clone().unwrap();
        for (map, srgb) in [(colour, true), (surface, false), (normal, false)] {
            let image = loader.images.get(&map).unwrap();
            let data = image.data.as_ref().unwrap();
            assert!(data.len() >= 64 * 64 * 4);
            assert!(data.iter().any(|&b| b != 0));
            assert_eq!(
                image.texture_descriptor.format
                    == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                srgb
            );
        }
        let image = loader
            .images
            .get(&material.diffuse_map.clone().unwrap())
            .unwrap();
        let data = &image.data.as_ref().unwrap()[..64 * 64 * 4];
        let pixels = data.as_chunks::<4>().0;
        assert!(pixels.iter().any(|p| p[..3] != data[..3]));
        assert!(pixels.iter().any(|p| p[3] == 255));
    }

    /// A bearded man takes one of the six beard maps; a race without the
    /// beard bit does not.
    #[test]
    fn a_bearded_man_takes_his_beard() {
        let record = CharacterRecord {
            profession_rest: 3 << 20,
            ..Default::default()
        };
        let exists = |name: &str| name.ends_with(".rfi");
        let job = SkinJob::of_record(2, "human", &record, 0x1BF, &[0.0; 12], &mut || 1, &exists);
        assert_eq!(
            job.beard,
            Some(["bearda03d.rfi".to_string(), "bearda03n.rfi".to_string()])
        );
        // The beard's colour is the picker's diffuse halfway to grey 0.3.
        let (diffuse, _) = exa_assets::hair::pick(0.0, 0.0, 0.0);
        assert_eq!(job.beard_colour, Some(diffuse.map(|c| c + (0.3 - c) * 0.5)));
        let no = SkinJob::of_record(2, "human", &record, 0x19F, &[0.0; 12], &mut || 1, &exists);
        assert!(no.beard.is_none());
    }
}
