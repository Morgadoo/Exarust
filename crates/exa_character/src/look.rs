//! The character look the builder makes from a record and a race: the skin
//! texture job (which texture set, tone, age darkening, beard and normal-map
//! blend), the hair style and which of its pieces show, and the two random
//! numbers the skin job takes from the record's look seed.
//!
//! The skin colour itself (the tone grid and the sRGB chain) is in
//! [`exa_assets::human_skin`]; the hair colour picker is in
//! [`exa_assets::hair`]. This module holds the parts that read the record
//! and race only, so they can be tested without an asset loader.

use exa_formats::characters::CharacterRecord;
use exa_formats::races::{LOOK_AGE_DARKENS, LOOK_BEARD, LOOK_FEMALE_TEXTURES, LOOK_MALE_TEXTURES};

/// The engine's random step: state × 0x0808_8405 + 1, wrapping in 32 bits.
pub const RANDOM_STEP: u32 = 0x0808_8405;

/// How far each body slot's limb may stretch in centimetres: the thigh and
/// shin slots (2, 3, 5 and 6) have a limit, the others do not.
pub const LIMB_STRETCH: [f32; 18] = [
    0.0, 55.0, 54.0, 0.0, 55.0, 54.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// The scale a body part of `slot` gets when the character's scale is
/// `scale` and the part's stored length is `length_cm` (the builder's stored
/// per-part length, half a 55 cm thigh in the shipped body).
///
/// A part with a stretch limit takes the character's scale; when that would
/// stretch the part more than its limit allows (the scale below twice the
/// length over the limit), the part keeps the mean of the two instead, so a
/// very short character's legs do not collapse.
pub fn limb_scale(scale: f32, slot: usize, length_cm: f32) -> f32 {
    let limit = LIMB_STRETCH
        .get(slot.wrapping_sub(1))
        .copied()
        .unwrap_or(0.0);
    if limit <= 1.0 {
        return scale;
    }
    let needed = (length_cm * 2.0) / limit;
    if scale < needed {
        (needed + scale) * 0.5
    } else {
        scale
    }
}

/// The next state of the engine's random generator.
pub fn random_step(state: u32) -> u32 {
    state.wrapping_mul(RANDOM_STEP).wrapping_add(1)
}

/// One draw in 0…1 as the original's float: the state as a double times
/// 2⁻³², narrowed to a float.
pub fn random_unit(state: u32) -> f32 {
    ((f64::from(state)) * (1.0 / 4_294_967_296.0)) as f32
}

/// A look seed: the record's own, or a fresh one when it holds zero. The
/// original's shipped records all store a non-zero word.
pub fn look_seed(seed: u32, mut fresh: impl FnMut() -> u32) -> u32 {
    if seed == 0 { fresh() } else { seed }
}

/// The two skin-map offsets the builder draws from the look seed, each
/// `draw − 0.5` in −0.5…0.5. The second draw follows the first from the
/// advanced state.
pub fn skin_uv_offset(seed: u32) -> [f32; 2] {
    let first = random_step(seed);
    let second = random_step(first);
    [random_unit(first) - 0.5, random_unit(second) - 0.5]
}

/// The race's skin texture set the character draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinSet {
    /// No texture set: the race's body keeps its own skin material.
    None,
    /// The men's `difm`, `spcm`, `nrmm0…5`.
    Male,
    /// The women's `diff`, `spcf`, `nrmf0…5`.
    Female,
}

/// The texture set a race of `look` gives a character of `female` sex:
/// the women's when the race has the women's set (look bit 0), else the
/// men's when it has that (bit 1), else none.
pub fn skin_set(look: u32, female: bool) -> SkinSet {
    if look & LOOK_FEMALE_TEXTURES != 0 {
        if female {
            SkinSet::Female
        } else {
            SkinSet::Male
        }
    } else if look & LOOK_MALE_TEXTURES != 0 {
        SkinSet::Male
    } else {
        SkinSet::None
    }
}

/// The race's texture names for a set: colour, gloss and the six normal
/// maps, all named after the body model's stem (`human` → `humandifm`).
pub fn skin_textures(stem: &str, set: SkinSet) -> Option<SkinTextures> {
    let (colour, gloss, normal) = match set {
        SkinSet::None => return None,
        SkinSet::Male => ("difm", "spcm", "nrmm"),
        SkinSet::Female => ("diff", "spcf", "nrmf"),
    };
    Some(SkinTextures {
        colour: format!("{stem}{colour}"),
        gloss: format!("{stem}{gloss}"),
        normals: std::array::from_fn(|i| format!("{stem}{normal}{i}")),
    })
}

/// A race's skin textures: its colour map, gloss map and six normal maps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkinTextures {
    pub colour: String,
    pub gloss: String,
    pub normals: [String; 6],
}

/// The beard textures of one of the six beards: its colour (`…d.rfi`) and
/// normal (`…n.rfi`) maps. The original names them `bearda01d.rfi` to
/// `bearda06n.rfi`.
pub fn beard_textures(index: usize) -> Option<[String; 2]> {
    (0..6).contains(&index).then(|| {
        [
            format!("bearda0{}d.rfi", index + 1),
            format!("bearda0{}n.rfi", index + 1),
        ]
    })
}

/// The beard a record wears: a man of a race with beards (look bit 5)
/// whose record names one in bits 20–23 of its profession word. The stored
/// number is one more than the texture index; six beards are shipped.
pub fn beard_index(record: &CharacterRecord, look: u32) -> Option<usize> {
    let stored = (record.profession_rest & 0x00F0_0000) >> 20;
    (look & LOOK_BEARD != 0 && !record.female && stored != 0 && stored <= 6)
        .then(|| (stored - 1) as usize)
}

/// One blended normal map of the skin job: how strongly it shows and its
/// texture.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalLayer {
    pub amount: f32,
    pub texture: String,
}

/// The skin job's normal-map layers, from the morph controller's weights.
///
/// The controller's weights are laid out with six entries per sex at
/// `weights[sex * 6…]` (the builder reads them with a stride of six and
/// picks the first four); the special layer takes `weights[4]` for either
/// sex. A layer shows only when its weight is over 0.01 in double
/// precision **and** the race has that map. A feral character adds its
/// feral level to the first layer and can show the extra `nrm?5` map on its
/// own. At most five layers are kept (four plus the extra).
pub fn normal_layers(
    weights: &[f32],
    sex: usize,
    feral: f32,
    normals: &[String; 6],
    has_extra: bool,
) -> Vec<NormalLayer> {
    let mut out = Vec::new();
    let at = |i: usize| weights.get(i).copied().unwrap_or(0.0);
    let base = sex * 6;
    for (i, texture) in normals.iter().take(4).enumerate() {
        let w = at(base + i);
        if f64::from(w) > 0.01 && !texture.is_empty() {
            let amount = if i == 0 && feral > 0.0 { w + feral } else { w };
            out.push(NormalLayer {
                amount,
                texture: texture.clone(),
            });
        }
    }
    let age = at(4);
    if (f64::from(age) > 0.01 || f64::from(feral) > 0.01) && has_extra {
        out.push(NormalLayer {
            amount: feral * 1.0 + age,
            texture: normals[5].clone(),
        });
    }
    out.truncate(5);
    out
}

/// The morph controller's normal weights the original's skin job reads,
/// derived from a physique's eleven target weights. The men's first four
/// corners sit at 0–3, the age at 4; the women's four corners at 6–9.
/// **Guess:** the controller's six-per-sex layout is not in the pack; the
/// values are copied from the physique targets, which the shipped morph
/// sets use for the same features.
pub fn controller_weights(weights: &[f32]) -> [f32; 12] {
    let w = |i: usize| weights.get(i).copied().unwrap_or(0.0);
    let mut out = [0.0; 12];
    out[..4].copy_from_slice(&[w(0), w(1), w(2), w(3)]);
    out[4] = w(4);
    out[5] = w(5);
    out[6..10].copy_from_slice(&[w(0), w(1), w(2), w(3)]);
    out
}

/// The age darkening the skin job stores: the feral level when the race
/// darkens with age (look bit 8) and the character is feral or aged. The
/// skin shader darkens the colour by it.
pub fn skin_darkening(record: &CharacterRecord, look: u32) -> Option<f32> {
    (look & LOOK_AGE_DARKENS != 0 && (record.feral > 0.0 || record.age > 0.0))
        .then_some(record.feral)
}

/// The hair entry a style picks: styles count from 1 in the record, entry 0
/// of `CharExt` is style 1 and style 0 is bald.
pub fn hair_entry(style: u32, entries: &[String]) -> Option<&String> {
    if style == 0 {
        return None;
    }
    entries.get(style as usize - 1)
}

/// Whether a hair root shows: the `?`-prefixed pieces show only while the
/// head is covered (the helmet's coverage sets the head's hide bit), and
/// every other piece shows otherwise.
pub fn hair_root_kept(name: &str, head_covered: bool) -> bool {
    name.starts_with('?') == head_covered
}

/// What a hair refresh must do, as the builder's rebuild cut: keep the hair
/// (0), show only its `?` pieces (1) or replace it whole (2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HairCut {
    Keep,
    Cover,
    Replace,
}

/// The refresh a new look needs: unchanged when the style and coverage match
/// the built hair, covered when only the coverage changed, replaced when the
/// style changed.
pub fn hair_refresh(
    built_style: u32,
    built_covered: bool,
    style: u32,
    covered: bool,
    has_hair: bool,
) -> HairCut {
    if !has_hair || built_style != style {
        HairCut::Replace
    } else if built_covered != covered {
        HairCut::Cover
    } else {
        HairCut::Keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator: 0 → 1 → 0x08088406 → …, and a draw is the state over
    /// 2³² less a half.
    #[test]
    fn the_look_seed_draws_the_engine_sequence() {
        assert_eq!(random_step(0), 1);
        assert_eq!(random_step(1), 0x0808_8406);
        assert_eq!(random_step(u32::MAX), 0xF7F7_7BFC);
        assert_eq!(random_unit(0), 0.0);
        assert_eq!(random_unit(0x8000_0000), 0.5);
        // The double narrows to one in a float, as the engine's cast does.
        assert_eq!(random_unit(u32::MAX), 1.0);
        // Seed 1: the first draw is 0x08088406 / 2³² − ½.
        let offset = skin_uv_offset(1);
        assert_eq!(offset[0], random_unit(0x0808_8406) - 0.5);
        assert_eq!(offset[1], random_unit(random_step(0x0808_8406)) - 0.5);
    }

    /// A 27.5 cm stored thigh over the 55 cm limit needs scale 1.0 (the
    /// builder's two-times rule); a 0.5 character's part keeps the mean,
    /// 0.75, rather than collapsing to half length.
    #[test]
    fn a_short_characters_legs_keep_half_the_required_scale() {
        assert_eq!(limb_scale(1.0, 2, 27.5), 1.0);
        assert_eq!(limb_scale(0.5, 2, 27.5), 0.75);
        assert_eq!(limb_scale(0.5, 3, 27.0), 0.75);
        assert_eq!(limb_scale(0.5, 5, 27.5), 0.75);
        assert_eq!(limb_scale(0.5, 6, 27.0), 0.75);
        // A part whose limit is not reached keeps the body scale.
        assert_eq!(limb_scale(0.8, 2, 20.0), 0.8);
        assert_eq!(limb_scale(1.9, 2, 27.5), 1.9);
        // Slots without a limit and the pelvis are unchanged.
        assert_eq!(limb_scale(0.9, 1, 200.0), 0.9);
        assert_eq!(limb_scale(0.9, 9, 200.0), 0.9);
    }

    #[test]
    fn a_zero_seed_is_replaced_once() {
        assert_eq!(look_seed(0xDEAD_BEEF, || 7), 0xDEAD_BEEF);
        assert_eq!(look_seed(0, || 7), 7);
    }

    #[test]
    fn the_race_picks_the_texture_set() {
        assert_eq!(skin_set(0x1BF, false), SkinSet::Male);
        assert_eq!(skin_set(0x1BF, true), SkinSet::Female);
        assert_eq!(skin_set(0x2, true), SkinSet::Male);
        assert_eq!(skin_set(0x0, true), SkinSet::None);
        let human = skin_textures("human", SkinSet::Male).unwrap();
        assert_eq!(human.colour, "humandifm");
        assert_eq!(human.gloss, "humanspcm");
        assert_eq!(human.normals[0], "humannrmm0");
        assert_eq!(human.normals[5], "humannrmm5");
        let woman = skin_textures("human", SkinSet::Female).unwrap();
        assert_eq!(
            (woman.colour.as_str(), woman.gloss.as_str()),
            ("humandiff", "humanspcf")
        );
        assert_eq!(woman.normals[5], "humannrmf5");
        assert_eq!(skin_textures("human", SkinSet::None), None);
    }

    #[test]
    fn the_beards_are_the_six_shipped_maps() {
        assert_eq!(
            beard_textures(0).unwrap(),
            ["bearda01d.rfi".to_string(), "bearda01n.rfi".to_string()]
        );
        assert_eq!(beard_textures(5).unwrap()[1], "bearda06n.rfi");
        assert_eq!(beard_textures(6), None);
    }

    #[test]
    fn a_beard_needs_its_race_bit_a_man_and_a_stored_number() {
        let mut r = CharacterRecord {
            profession_rest: 3 << 20,
            ..Default::default()
        };
        assert_eq!(beard_index(&r, LOOK_BEARD), Some(2));
        assert_eq!(beard_index(&r, 0x1BF & !LOOK_BEARD), None);
        r.female = true;
        assert_eq!(beard_index(&r, LOOK_BEARD), None);
        r.female = false;
        r.profession_rest = 7 << 20;
        assert_eq!(beard_index(&r, LOOK_BEARD), None);
        r.profession_rest = 0;
        assert_eq!(beard_index(&r, LOOK_BEARD), None);
    }

    /// A physique at the male middle has weight 1 on the age target; a
    /// layer shows above 0.01 and a hidden race map is skipped.
    #[test]
    fn the_normal_layers_follow_the_weights_and_the_threshold() {
        let names: [String; 6] = std::array::from_fn(|i| format!("humannrmm{i}"));
        let mut weights = [0.0f32; 12];
        weights[0] = 0.5;
        weights[1] = 0.02;
        weights[2] = 0.01;
        weights[3] = 1.0;
        weights[4] = 0.03;
        let layers = normal_layers(&weights, 0, 0.0, &names, true);
        assert_eq!(
            layers,
            vec![
                NormalLayer {
                    amount: 0.5,
                    texture: "humannrmm0".into()
                },
                NormalLayer {
                    amount: 0.02,
                    texture: "humannrmm1".into()
                },
                NormalLayer {
                    amount: 1.0,
                    texture: "humannrmm3".into()
                },
                NormalLayer {
                    amount: 0.03,
                    texture: "humannrmm5".into()
                },
            ]
        );
        // 0.01 itself is not over the threshold.
        let mut only = [0.0f32; 12];
        only[2] = 0.010_000_001;
        assert_eq!(normal_layers(&only, 0, 0.0, &names, false).len(), 1);
        only[2] = 0.01;
        assert!(normal_layers(&only, 0, 0.0, &names, false).is_empty());
        // The extra map shows for the feral level alone; its amount adds it.
        assert_eq!(
            normal_layers(&only, 0, 0.2, &names, true),
            vec![NormalLayer {
                amount: 0.200_000_01,
                texture: "humannrmm5".into()
            }]
        );
        // A missing texture is skipped.
        let missing: [String; 6] = std::array::from_fn(|i| {
            if i == 1 {
                String::new()
            } else {
                format!("humannrmm{i}")
            }
        });
        assert_eq!(normal_layers(&weights, 0, 0.0, &missing, true).len(), 3);
    }

    /// The women's four layers come from indices 6–9.
    #[test]
    fn the_womens_layers_move_six_places() {
        let names: [String; 6] = std::array::from_fn(|i| format!("humannrmf{i}"));
        let mut weights = [0.0f32; 12];
        weights[0] = 1.0;
        weights[6] = 0.25;
        weights[7] = 0.5;
        let layers = normal_layers(&weights, 1, 0.0, &names, false);
        assert_eq!(
            (layers[0].amount, layers[0].texture.as_str()),
            (0.25, "humannrmf0")
        );
        assert_eq!(
            (layers[1].amount, layers[1].texture.as_str()),
            (0.5, "humannrmf1")
        );
        assert_eq!(layers.len(), 2);
    }

    #[test]
    fn the_skin_darkens_only_a_feral_or_aged_race_with_the_bit() {
        let mut r = CharacterRecord::default();
        assert_eq!(skin_darkening(&r, LOOK_AGE_DARKENS), None);
        r.feral = 0.5;
        assert_eq!(skin_darkening(&r, LOOK_AGE_DARKENS), Some(0.5));
        assert_eq!(skin_darkening(&r, 0), None);
        r.feral = 0.0;
        r.age = 0.3;
        assert_eq!(skin_darkening(&r, LOOK_AGE_DARKENS), Some(0.0));
    }

    #[test]
    fn a_hair_style_counts_from_one_and_zero_is_bald() {
        let entries: Vec<String> = ["hair hula01", "hair hula02"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(hair_entry(0, &entries), None);
        assert_eq!(hair_entry(1, &entries).unwrap(), "hair hula01");
        assert_eq!(hair_entry(2, &entries).unwrap(), "hair hula02");
        assert_eq!(hair_entry(3, &entries), None);
    }

    #[test]
    fn question_pieces_show_only_under_a_covered_head() {
        assert!(!hair_root_kept("?fringe", false));
        assert!(hair_root_kept("?fringe", true));
        assert!(hair_root_kept("hairmain", false));
        assert!(!hair_root_kept("hairmain", true));
    }

    #[test]
    fn a_look_refresh_keeps_covers_or_replaces_the_hair() {
        assert_eq!(hair_refresh(0, false, 0, false, false), HairCut::Replace);
        assert_eq!(hair_refresh(3, false, 3, false, true), HairCut::Keep);
        assert_eq!(hair_refresh(3, false, 3, true, true), HairCut::Cover);
        assert_eq!(hair_refresh(3, false, 4, false, true), HairCut::Replace);
    }

    #[test]
    fn the_controllers_weights_take_the_physiques() {
        let w: Vec<f32> = (0..11).map(|i| i as f32).collect();
        let c = controller_weights(&w);
        assert_eq!(&c[..6], &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(&c[6..10], &[0.0, 1.0, 2.0, 3.0]);
        assert_eq!(c[10], 0.0);
    }

    /// The installed races' look words: Human 0x1BF, Ancient 0x86,
    /// Skeleton 4.
    #[test]
    fn the_installed_races_look_words() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
        let races = exa_formats::races::parse(resource.get("races.rdb").unwrap()).unwrap();
        let look = |name: &str| races.iter().find(|r| r.name == name).unwrap().look;
        assert_eq!(look("Human"), 0x1BF);
        assert_eq!(look("Ancient"), 0x86);
        assert_eq!(look("Skeleton"), 4);
    }

    /// The look bits name the shipped races' features.
    #[test]
    fn the_shipped_races_look_words() {
        let human = exa_formats::races::LOOK_HAIR
            | exa_formats::races::LOOK_BEARD
            | exa_formats::races::LOOK_SKIN
            | exa_formats::races::LOOK_AGE_DARKENS
            | LOOK_FEMALE_TEXTURES
            | LOOK_MALE_TEXTURES;
        // Human 0x1BF carries all of them.
        assert_eq!(human, 0x1B3);
    }
}
