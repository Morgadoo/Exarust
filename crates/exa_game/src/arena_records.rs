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
//! A generated opponent writes personality `[-100, 100, 0, 0]`, a hire-list
//! opponent writes none, and both are given the event's 100/100 condition
//! by the spawn plan, not by the record. The event weapon is put in the
//! record's main hand before the outfit pass, and the profile's flag 4 keeps
//! the outfit from picking another weapon.
//!
//! Evidence: the generic and hire-list spawn routines' record calls and the
//! record writer's field order (arena research notes, `docs/formats/RFC.md`).

use exa_formats::characters::{CharacterRecord, ItemRef, SKILLS};

use crate::arena_names::{FEMALE_NAMES, MALE_NAMES};
use crate::arena_outfit::{
    GeneratedOutfit, Lcg, NameTable, NpcProfile, NpcRequest, OutfitCharacter, OutfitTables,
    ProfessionCatalog, generate_by_level_lcg, generate_profile_lcg, random_npc_lcg,
};
use crate::arena_spawn::{GenericPlan, HirePlan, OutfitProfile};

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
        }
    }

    /// A hire-list opponent: its six-level record and eight-byte outfit
    /// profile, with the event's drawn weapon in the main hand.
    #[must_use]
    pub fn hire(plan: HirePlan, weapon: Option<u32>) -> Self {
        Self {
            level: plan.stat_level,
            outfit: OutfitSource::Profile(plan.outfit),
            traits: [0.0; 4],
            weapon,
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
    record.hands[0] = fighter
        .weapon
        .map_or(ItemRef::NONE, |id| ItemRef::new(id, 0));
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
    record.voice_number = profile.appearance;
    record.voice = exa_formats::characters::voice_name(profile.appearance);
    record.voice_pitch = 1.0;
    record.set_profession(look.flags);
    record.combat_skill = profile.skill;
    record.undead_voice = profile.variant;
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
        assert!(previous > 0 || record.undead_voice == 0);
    }

    #[test]
    fn the_hire_forced_outfit_is_the_profile_bytes() {
        let plan = hire_plan(3, 2, 0);
        assert_eq!(plan.outfit.flags(), 0x0b);
        assert_eq!(&plan.outfit.0[5..8], &[66, 140, 1]);
        let fighter = GeneratedFighter::hire(plan, None);
        assert_eq!(fighter.outfit, OutfitSource::Profile(plan.outfit));
        assert_eq!(fighter.traits, [0.0; 4]);
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
