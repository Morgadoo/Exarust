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
    NormalLayer, SkinSet, beard_index, beard_textures, normal_layers, skin_darkening, skin_set,
    skin_textures, skin_uv_offset,
};

/// The generator's skin job.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinJob {
    pub format: SkinFormat,
    /// The race's colour and gloss maps, when it has a skin set.
    pub colour: Option<String>,
    pub gloss: Option<String>,
    /// The race's six normal maps, by physique index.
    pub normals: [Option<String>; 6],
    /// The record's two tone words.
    pub tone: [u16; 2],
    /// The two skin-map offsets drawn from the look seed.
    pub uv_offset: [f32; 2],
    /// The feral level the skin darkens by, when it does.
    pub darkening: Option<f32>,
    /// The beard's colour and normal maps.
    pub beard: Option<[String; 2]>,
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
            format: SkinFormat::of_setting(pixel_format),
            colour: None,
            gloss: None,
            normals: std::array::from_fn(|_| None),
            tone: [0; 2],
            uv_offset: [0.0; 2],
            darkening: None,
            beard: None,
            layers: Vec::new(),
            first_apparel: None,
            started: false,
        }
    }

    /// Fills the job's look from a record and race, if the
    /// race has a skin set. `exists` says whether a texture the race names
    /// is shipped.
    pub fn with_look(
        mut self,
        record: &CharacterRecord,
        look_word: u32,
        controller_weights: &[f32],
        exists: &dyn Fn(&str) -> bool,
    ) -> Self {
        self.tone = record.skin;
        self.uv_offset = skin_uv_offset(record.search_seed);
        self.darkening = skin_darkening(record, look_word);
        self.beard = beard_index(record, look_word)
            .and_then(beard_textures)
            .filter(|b| exists(&b[0]) && exists(&b[1]));
        let _ = (controller_weights, exists);
        self
    }

    /// Fills the race texture names for a body stem.
    pub fn with_stem(mut self, stem: &str, set: SkinSet) -> Self {
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

    /// Resets the job for a new texture pass.
    pub fn reset(&mut self) {
        self.colour = None;
        self.gloss = None;
        self.normals = std::array::from_fn(|_| None);
        self.layers.clear();
        self.beard = None;
        self.first_apparel = None;
        self.started = false;
    }

    /// The job with a look, in one call: the stem's textures, the record's
    /// tone, seed, darkening and beard, and the layers whose maps ship.
    pub fn of_record(
        pixel_format: u32,
        stem: &str,
        record: &CharacterRecord,
        look_word: u32,
        controller_weights: &[f32],
        exists: &dyn Fn(&str) -> bool,
    ) -> Self {
        let set = skin_set(look_word, record.female);
        let mut job = Self::new(pixel_format).with_stem(stem, set).with_look(
            record,
            look_word,
            controller_weights,
            exists,
        );
        // The normal layer builder ran before the stem in `with_look`;
        // rebuild the layers now that the names are known.
        let count = record.female as usize;
        job.layers = normal_layers(
            controller_weights,
            count,
            record.feral,
            &job.normal_names(),
            job.normals[5].is_some(),
        )
        .into_iter()
        .filter(|l| exists(&l.texture))
        .collect();
        job
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        job.reset();
        assert!(job.colour.is_none() && job.beard.is_none() && !job.started);
        assert_eq!(job.format, SkinFormat::of_setting(2));
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
        let job = SkinJob::of_record(3, "human", &record, 0x1BF, &[0.0; 12], &exists);
        assert_eq!(job.format.side, 0x400);
        assert_eq!(job.colour.as_deref(), Some("humandiff"));
        assert_eq!(job.gloss.as_deref(), Some("humanspcf"));
        assert_eq!(job.normals[5].as_deref(), Some("humannrmf5"));
        assert_eq!(job.tone, [0x8000, 0x8000]);
        assert_eq!(job.uv_offset, skin_uv_offset(1));
        assert!(job.beard.is_none());
        assert!(job.layers.is_empty());
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
        let job = SkinJob::of_record(2, "human", &record, 0x1BF, &[0.0; 12], &exists);
        assert_eq!(
            job.beard,
            Some(["bearda03d.rfi".to_string(), "bearda03n.rfi".to_string()])
        );
        let no = SkinJob::of_record(2, "human", &record, 0x19F, &[0.0; 12], &exists);
        assert!(no.beard.is_none());
    }
}
