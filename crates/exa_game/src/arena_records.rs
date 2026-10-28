//! The generated fighters' character records and outfits.
//!
//! The arena writes a fighter's own record before it creates the body: the
//! random-NPC pass fills the look, profession, name and skill rows, then the
//! outfit pass dresses it from the arena's item pools. A generic opponent
//! takes the level-derived outfit; a hire-list opponent takes its eight-byte
//! profile, with the event's company bytes forced and the event's weapon
//! already in the main hand. The body generator then dresses the merged
//! character mesh from the record's worn list and hands, exactly as a placed
//! character is dressed.
//!
//! The record fields this module writes are Exarust's mapping of the
//! engine's runtime record: the two physique values are the look's frame and
//! height, the record's height is the look's weight deviation and its age
//! the look's strength value (the original writes them to those runtime
//! fields). The name, surname, skin words, hair style and colour, voice
//! number, profession word, skill, variant word, search seed and the four
//! personality values follow the same "too many fields" shape.
//!
//! Both a generated and a hire-list opponent write personality
//! `[-100, 100, 0]` after the outfit pass (the fourth value stays 0), and
//! both are given the event's 100/100 condition by the spawn plan, not by
//! the record. The event weapon is put in the
//! record's main hand before the outfit pass, and the profile's flag 4 keeps
//! the outfit from picking another weapon.
//!
//! The random NPC pass's variant (the draw that avoids the previous
//! fighter's) is the record's voice number, its 0.9…1.07 curve the voice
//! pitch, and its appearance draw the undead voice set: the runtime record
//! keeps voice, pitch and profession side by side, as the file does, and
//! the undead voice beside the combat skill.
//!
//! The Beast ([`beast_record`]) is not a random NPC: its record is the
//! Ogre race (profession word 0x300), a combat skill of 100, its tier's
//! hand item and worn pieces, a voice pitch from one smooth draw, voice 0
//! and 50000 experience spent on the profession grid.
//!
//! Every generated record starts from a sequence the spawn reseeds from the
//! performance counter (its low 32 bits byte-reversed), as the original does
//! before each generated body.
//!
//! Evidence: the generic, hire-list and beast spawn routines' record calls
//! and the record writer's field order (arena research notes,
//! `docs/formats/RFC.md`).

use exa_formats::characters::{CharacterRecord, ItemRef, SKILLS};

use crate::arena_names::{FEMALE_NAMES, MALE_NAMES};
use crate::arena_outfit::{
    GeneratedOutfit, Lcg, NameTable, NpcProfile, NpcRequest, OutfitCharacter, OutfitTables,
    ProfessionCatalog, generate_by_level_lcg, generate_profile_lcg, random_npc_lcg,
};
use crate::arena_spawn::{GenericPlan, HirePlan, OutfitProfile, beast_plan, pick_weapon_slot};

/// The wardrobe a generated fighter is dressed from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutfitSource {
    /// The numeric-level pass: its level, the height bias (`level × 0.05`)
    /// and the mode-flag word.
    Level {
        level: i32,
        warmth: f32,
        mode_flags: u32,
    },
    /// The eight-byte profile pass (a hire-list opponent).
    Profile(OutfitProfile),
}

/// A fighter the generator writes a record for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratedFighter {
    /// The character level the random-NPC pass receives.
    pub level: i32,
    /// How the outfit is chosen.
    pub outfit: OutfitSource,
    /// The four personality values, −100…100.
    pub traits: [f32; 4],
    /// An item id already in the main hand before the outfit pass.
    pub weapon: Option<u32>,
    /// The event weapon row's three variants. When set, the variant is
    /// drawn from the shared sequence after the random NPC pass and before
    /// the outfit pass (the original's order), and it replaces `weapon`.
    pub weapon_row: Option<[u32; 3]>,
}

impl GeneratedFighter {
    /// A generic opponent: the level-derived outfit, the level's warmth bias
    /// and the original's personality pair.
    #[must_use]
    pub fn generic(plan: GenericPlan) -> Self {
        Self {
            level: plan.level,
            outfit: OutfitSource::Level {
                level: plan.outfit_level,
                warmth: plan.warmth,
                mode_flags: 0,
            },
            traits: [-100.0, 100.0, 0.0, 0.0],
            weapon: None,
            weapon_row: None,
        }
    }

    /// A hire-list opponent: its six-level record and eight-byte outfit
    /// profile, with the event's drawn weapon in the main hand. It writes
    /// the same personality as a generic opponent.
    #[must_use]
    pub fn hire(plan: HirePlan, weapon: Option<u32>) -> Self {
        Self {
            level: plan.stat_level,
            outfit: OutfitSource::Profile(plan.outfit),
            traits: [-100.0, 100.0, 0.0, 0.0],
            weapon,
            weapon_row: None,
        }
    }

    /// A hire-list opponent whose weapon is drawn from the event row during
    /// the generation, as the original's hire spawn does: after the random
    /// NPC pass, one step of the shared sequence picks the row's slot.
    #[must_use]
    pub fn hire_from_row(plan: HirePlan, row: [u32; 3]) -> Self {
        Self {
            weapon_row: Some(row),
            ..Self::hire(plan, None)
        }
    }
}

/// Write a generated fighter's record.
///
/// `lcg` is the arena's shared sequence: every draw the look and outfit
/// passes make advances it, in the original's order. `previous` is the
/// variant index the look pass avoids repeating; it is updated to this
/// fighter's. The caller supplies the outfit tables built from the object
/// database.
pub fn generated_record(
    fighter: &GeneratedFighter,
    lcg: &mut Lcg,
    tables: &OutfitTables,
    professions: &mut ProfessionCatalog,
    previous: &mut u32,
) -> CharacterRecord {
    // The original's name tables hold sixteen bytes per row with only the
    // first name filled. Building the paired rows once per generation keeps
    // the retail names out of the outfit module, as its API expects.
    let male: Vec<(&str, &str)> = MALE_NAMES.iter().map(|name| (*name, "")).collect();
    let female: Vec<(&str, &str)> = FEMALE_NAMES.iter().map(|name| (*name, "")).collect();
    let names = NameTable {
        male: &male,
        female: &female,
    };
    let request = NpcRequest {
        level: fighter.level,
        forced_flags: 0,
        warmth: 0.0,
        previous_variant: *previous,
        names: &names,
    };
    let profile = random_npc_lcg(lcg, &request, professions);
    *previous = profile.variant;
    // The hire spawn draws the event weapon after the random NPC pass and
    // before the outfit pass, from the same sequence.
    let weapon = match fighter.weapon_row {
        Some(row) => {
            let roll = lcg.step();
            Some(pick_weapon_slot(row, roll)).filter(|&id| id != 0)
        }
        None => fighter.weapon,
    };
    let character = OutfitCharacter {
        // The premade-skill probe reads a skill technique this generated
        // record never has.
        skill_word: 0,
        // The outfit pass reads the fourth profession row's experience word.
        experience: u32::from(profile.professions.rows[3].experience),
        warmth: 0.0,
        record_flags: profile.look.flags,
    };
    let outfit = match fighter.outfit {
        OutfitSource::Level {
            level,
            warmth,
            mode_flags,
        } => generate_by_level_lcg(level, warmth, mode_flags, lcg, tables, &character),
        OutfitSource::Profile(profile_bytes) => {
            generate_profile_lcg(&profile_bytes.0, lcg, tables, &character)
        }
    };
    let mut record = CharacterRecord::new();
    apply_profile(&mut record, &profile);
    for (index, skill) in record.skills.iter_mut().enumerate().take(4) {
        let row = profile.professions.rows[index];
        skill.points = row.experience;
        for (rank, &column) in row.levels.iter().enumerate().take(6) {
            skill.techniques[rank] = column;
        }
    }
    record.traits = fighter.traits;
    record.hands[0] = weapon.map_or(ItemRef::NONE, |id| ItemRef::new(id, 0));
    apply_outfit(&mut record, &outfit, tables);
    record
}

/// Write the look fields a random NPC's profile gives a record.
fn apply_profile(record: &mut CharacterRecord, profile: &NpcProfile<'_>) {
    let look = profile.look;
    record.name = profile.name.0.to_string();
    record.surname = profile.name.1.to_string();
    record.physique = [look.frame, look.height];
    record.height = look.weight;
    record.age = look.strength;
    record.skin = [look.face_seed, look.voice_seed];
    record.hair_style = look.build_seed;
    record.hair_colour = [look.feature_seed, look.skin_tone, look.hair_tone, 0];
    record.face = [0, 0];
    // The variant (drawn not to repeat the previous fighter's) is the voice
    // number and the 0.9…1.07 muscle curve the voice pitch; the appearance
    // draw is the undead voice set, beside the combat skill.
    record.voice_number = profile.variant;
    record.voice = exa_formats::characters::voice_name(profile.variant);
    record.voice_pitch = look.muscle;
    record.set_profession(look.flags);
    record.combat_skill = profile.skill;
    record.undead_voice = profile.appearance;
    record.search_seed = look.damage_seed;
    record.experience = 0;
    record.extra_word = 0;
    debug_assert_eq!(record.skills.len(), SKILLS);
}

/// Write the outfit's hands and worn pieces into the record.
///
/// A weapon or shield the outfit put in a hand is not worn again. The item
/// quality and condition the outfit pass computes belong to the item
/// instance in the original; Exarust's record stores only the id and flags,
/// so a body shows the object database's own appearance instead.
fn apply_outfit(record: &mut CharacterRecord, outfit: &GeneratedOutfit, tables: &OutfitTables) {
    let hands: Vec<u32> = outfit
        .hands
        .iter()
        .flatten()
        .map(|held| held.variant)
        .collect();
    for item in &outfit.items {
        if hands.contains(&item.id.0) || tables.facts(item.id).is_none() {
            continue;
        }
        record.worn.push(ItemRef::new(item.id.0, 0));
    }
    for (slot, held) in outfit.hands.iter().enumerate() {
        if let Some(held) = held {
            record.hands[slot] = ItemRef::new(held.variant, 0);
        }
    }
}

/// The experience the Beast's record spends on the profession grid.
pub const BEAST_EXPERIENCE: u32 = 50_000;
/// The Beast's combat skill.
pub const BEAST_COMBAT_SKILL: f32 = 100.0;

/// Write the Beast's record at `tier` from the reseeded shared sequence.
///
/// The original's order: the profession word 0x300 (race byte 3, the Ogre),
/// the combat skill 100, the base hand item 0x44, then one smooth draw for
/// the voice pitch `0.9 + 0.17 · smooth` (in double precision, narrowed
/// once); tiers three to five replace the hand item and add their four worn
/// pieces in order; the voice number is cleared and 50000 experience is
/// spent on the profession grid, which steps the same sequence. The
/// personality is the opponents' `-100, 100, 0`.
pub fn beast_record(
    tier: u32,
    lcg: &mut Lcg,
    professions: &mut ProfessionCatalog,
) -> CharacterRecord {
    let smooth = crate::arena_outfit::smooth_unit(lcg.unit());
    let plan = beast_plan(tier, smooth);
    let mut record = CharacterRecord::new();
    record.set_profession(plan.profession);
    record.combat_skill = BEAST_COMBAT_SKILL;
    record.hands[0] = ItemRef::new(plan.body, 0);
    record.voice_pitch = plan.voice_pitch;
    for &item in &plan.equipment {
        record.worn.push(ItemRef::new(item, 0));
    }
    record.voice_number = 0;
    let mut progress = crate::arena_outfit::ProfessionProgress::default();
    crate::arena_outfit::grant_profession_levels(lcg, professions, &mut progress, plan.ability);
    for (index, skill) in record.skills.iter_mut().enumerate().take(4) {
        let row = progress.rows[index];
        skill.points = row.experience;
        for (rank, &column) in row.levels.iter().enumerate().take(6) {
            skill.techniques[rank] = column;
        }
    }
    record.traits = [-100.0, 100.0, 0.0, 0.0];
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_outfit::ItemFacts;
    use crate::arena_spawn::{generic_plan, hire_plan};
    use exa_formats::characters::SKILLS;

    fn tables() -> OutfitTables {
        // A synthetic catalogue: every table row names one of two apparel
        // pieces, so the record's worn list is non-empty and pinned.
        OutfitTables::build(|id| {
            Some(ItemFacts {
                id,
                rank_word: 0,
                quality: 40,
                condition: 0,
                ..ItemFacts::default()
            })
        })
    }

    #[test]
    fn a_generic_fighter_takes_its_level_and_personality() {
        let fighter = GeneratedFighter::generic(generic_plan(3, 2));
        assert_eq!(fighter.level, 3);
        assert_eq!(
            fighter.outfit,
            OutfitSource::Level {
                level: 2,
                warmth: (3.0f64 * 0.05) as f32,
                mode_flags: 0
            }
        );
        assert_eq!(fighter.traits, [-100.0, 100.0, 0.0, 0.0]);
        assert_eq!(fighter.weapon, None);
    }

    #[test]
    fn a_hire_fighter_takes_its_forced_profile_and_weapon() {
        let plan = hire_plan(4, 2, 5);
        let fighter = GeneratedFighter::hire(plan, Some(0x1234));
        assert_eq!(fighter.level, 6);
        assert_eq!(fighter.outfit, OutfitSource::Profile(plan.outfit));
        assert_eq!(fighter.weapon, Some(0x1234));
    }

    #[test]
    fn a_generated_record_fills_the_look_profile_and_skill_rows() {
        let tables = tables();
        let fighter = GeneratedFighter::generic(generic_plan(2, 2));
        let mut lcg = Lcg::new(1);
        let mut professions = ProfessionCatalog::default();
        let mut previous = 0;
        let record = generated_record(&fighter, &mut lcg, &tables, &mut professions, &mut previous);
        // Pinned to the shipped name tables and this synthetic catalogue.
        assert_eq!(record.name, "Thelmis");
        assert_eq!(record.surname, "");
        assert_eq!(record.physique[0].to_bits(), 0x3eea_1fc7);
        assert_eq!(record.physique[1].to_bits(), 0x3f43_7001);
        assert_eq!(record.height.to_bits(), 0xbd8e_f090);
        assert_eq!(record.age.to_bits(), 0x3ddc_3b64);
        assert_eq!(record.skin, [37_889, 48_293]);
        assert_eq!(record.hair_style, 9);
        assert_eq!(record.hair_colour, [84, 56, 45, 0]);
        // The variant is the voice number, the appearance draw the undead
        // voice and the muscle curve the pitch.
        assert_eq!(record.voice_number, 4);
        assert_eq!(record.voice, "vi05");
        assert!((0.9..=1.07).contains(&record.voice_pitch));
        assert_eq!(record.profession(), 0x1_0000);
        assert_eq!(record.combat_skill.to_bits(), 0x423d_c771);
        assert_eq!(record.undead_voice, 0);
        assert_eq!(record.search_seed, 0x07a5_0b31);
        assert_eq!(record.skills[0].points, 15_313);
        assert_eq!(record.skills[0].techniques, [1, 3, 4, 2, 0, 0]);
        assert_eq!(record.skills[2].points, 5_000);
        assert_eq!(
            record.worn.iter().map(|worn| worn.id).collect::<Vec<_>>(),
            [1360, 249, 216]
        );
        assert!(record.hands.iter().all(|hand| !hand.is_some()));
        assert_eq!(previous, 4);
        // The name comes from the retail tables and has no surname.
        assert!(!record.name.is_empty());
        assert!(record.surname.is_empty());
        // The two physique values are the look's frame and height, and the
        // record's own height and age the weight and strength.
        assert!((0.25..=0.75).contains(&record.physique[0]));
        assert!(record.physique[1] >= 0.0 && record.physique[1] <= 1.0);
        assert!(record.height >= -1.0 && record.height <= 1.0);
        assert!(record.age >= 0.0 && record.age <= 1.0);
        // The profession grade is the flags' high nibble; the sex bit is
        // mirrored into the record.
        assert_eq!(record.female, record.profession() & 1 != 0);
        assert_eq!(record.profession() & 0xff00, 0);
        assert!(record.combat_skill >= 40.0);
        assert!(record.voice.starts_with("vi"));
        assert_eq!(record.skills.len(), SKILLS);
        assert_eq!(record.traits, [-100.0, 100.0, 0.0, 0.0]);
        assert_eq!(previous, record.voice_number);
    }

    #[test]
    fn a_row_drawn_weapon_follows_the_random_npc_pass() {
        // The variant is drawn from the step after the random NPC pass: a
        // second run that replays the pass and steps once picks the same
        // slot.
        let tables = tables();
        let plan = hire_plan(3, 2, 2);
        let row = [0x51, 0x52, 0x53];
        let fighter = GeneratedFighter::hire_from_row(plan, row);
        let mut lcg = Lcg::new(77);
        let mut professions = ProfessionCatalog::default();
        let mut previous = 0;
        let record = generated_record(&fighter, &mut lcg, &tables, &mut professions, &mut previous);
        let male: Vec<(&str, &str)> = MALE_NAMES.iter().map(|name| (*name, "")).collect();
        let female: Vec<(&str, &str)> = FEMALE_NAMES.iter().map(|name| (*name, "")).collect();
        let names = NameTable {
            male: &male,
            female: &female,
        };
        let mut replay = Lcg::new(77);
        let _ = random_npc_lcg(
            &mut replay,
            &NpcRequest {
                level: 6,
                forced_flags: 0,
                warmth: 0.0,
                previous_variant: 0,
                names: &names,
            },
            &mut ProfessionCatalog::default(),
        );
        let expected = pick_weapon_slot(row, replay.step());
        assert_eq!(record.hands[0].id, expected);
        assert!(row.contains(&record.hands[0].id));
    }

    #[test]
    fn the_beast_is_an_ogre_with_its_tier_items() {
        let mut lcg = Lcg::new(0x1234_5678);
        let mut professions = ProfessionCatalog::default();
        let record = beast_record(4, &mut lcg, &mut professions);
        // Race byte 3 (the Ogre), no sex bit, the combat skill of 100.
        assert_eq!(record.race, 3);
        assert!(!record.female);
        assert_eq!(record.profession(), 0x300);
        assert_eq!(record.combat_skill, 100.0);
        assert_eq!(record.hands[0].id, 0x207);
        assert_eq!(
            record.worn.iter().map(|worn| worn.id).collect::<Vec<_>>(),
            [0x210, 0x20e, 0x20f, 0x213]
        );
        assert_eq!(record.voice_number, 0);
        assert_eq!(record.traits, [-100.0, 100.0, 0.0, 0.0]);
        // The pitch is the first draw of the reseeded sequence, smoothed.
        let mut first = Lcg::new(0x1234_5678);
        let smooth = crate::arena_outfit::smooth_unit(first.unit());
        assert_eq!(
            record.voice_pitch.to_bits(),
            ((f64::from(smooth) * 0.17 + 0.9) as f32).to_bits()
        );
        // 50000 experience is spent: ten full ranks across the rows.
        let total: u32 = record
            .skills
            .iter()
            .map(|skill| u32::from(skill.points))
            .sum();
        assert_eq!(total, 50_000);
        // Tiers one and two keep the base item and wear nothing.
        let low = beast_record(1, &mut Lcg::new(1), &mut ProfessionCatalog::default());
        assert_eq!(low.hands[0].id, 0x44);
        assert!(low.worn.is_empty());
    }

    #[test]
    fn the_hire_forced_outfit_is_the_profile_bytes() {
        let plan = hire_plan(3, 2, 0);
        assert_eq!(plan.outfit.flags(), 0x0b);
        assert_eq!(&plan.outfit.0[5..8], &[66, 140, 1]);
        let fighter = GeneratedFighter::hire(plan, None);
        assert_eq!(fighter.outfit, OutfitSource::Profile(plan.outfit));
        // The hire routine writes the same -100, 100, 0 personality.
        assert_eq!(fighter.traits, [-100.0, 100.0, 0.0, 0.0]);
    }

    /// The installed object database dresses a generated fighter: its worn
    /// list names parseable apparel, the skill rows are filled and a hire's
    /// forced profile and weapon reach the record; skipped without
    /// `EXANIMA_DIR`.
    #[test]
    fn a_real_generated_fighter_dresses_from_the_object_database() {
        let Some(items) = real_items() else {
            return;
        };
        let facts: std::collections::BTreeMap<u32, ItemFacts> = items
            .iter()
            .map(|item| (item.id, ItemFacts::of_item(item)))
            .collect();
        let tables = OutfitTables::build(|id| facts.get(&id.0).cloned());
        let mut professions = ProfessionCatalog::default();
        let mut previous = 0;
        let mut lcg = Lcg::new(5_489);
        let generic = generated_record(
            &GeneratedFighter::generic(generic_plan(3, 2)),
            &mut lcg,
            &tables,
            &mut professions,
            &mut previous,
        );
        assert!(!generic.name.is_empty());
        assert!(!generic.worn.is_empty());
        assert!(
            generic
                .worn
                .iter()
                .all(|worn| items.iter().any(|item| item.id == worn.id)),
            "every worn id is in the object database"
        );
        assert!(generic.worn.iter().all(|worn| {
            items
                .iter()
                .find(|item| item.id == worn.id)
                .is_some_and(|item| !item.worn_model.is_empty())
        }));
        // A hire profile with the event's second row: the weapon is in the
        // main hand and the outfit does not replace it.
        let weapon = items
            .iter()
            .find(|item| item.weapon.is_some() && !item.model.is_empty())
            .map(|item| item.id)
            .expect("a shipped weapon");
        let fighter = GeneratedFighter::hire(hire_plan(3, 2, 1), Some(weapon));
        let record = generated_record(&fighter, &mut lcg, &tables, &mut professions, &mut previous);
        assert_eq!(record.hands[0].id, weapon);
        assert!(!record.worn.is_empty());
    }

    /// The installed object database's items, when `EXANIMA_DIR` names an
    /// installation.
    fn real_items() -> Option<Vec<exa_formats::objdb::Item>> {
        let dir = std::env::var("EXANIMA_DIR").ok()?;
        let data = exa_assets::GameData::open(std::path::Path::new(&dir)).ok()?;
        let bytes = data
            .pack(exa_assets::PackKind::Resource)?
            .get("objdb.rdb")?;
        let database = exa_formats::rdb::Database::parse(bytes).ok()?;
        Some(exa_formats::objdb::items(&database))
    }

    #[test]
    fn an_event_weapon_stays_in_the_main_hand_when_the_outfit_runs() {
        // A profile with flag 4: the outfit skips its weapon branch and
        // leaves the main hand alone.
        let tables = tables();
        let plan = hire_plan(3, 2, 3);
        let fighter = GeneratedFighter::hire(plan, Some(0x77));
        let mut lcg = Lcg::new(9);
        let mut professions = ProfessionCatalog::default();
        let mut previous = 0;
        let record = generated_record(&fighter, &mut lcg, &tables, &mut professions, &mut previous);
        assert_eq!(record.hands[0].id, 0x77);
    }
}
