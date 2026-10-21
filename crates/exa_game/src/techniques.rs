//! The technique selector's bridge to a roster member.
//!
//! The original's selector edits the character record's six skills (an
//! experience word and the technique bytes each) and the skill screen's
//! experience pool. A [`Member`] keeps the same: `skill_xp` and
//! `techniques` are the record's six skills, `xp` the pool.

use exa_ui::technique_selector::{
    SKILLS, SkillRecord, TechniqueChange, TechniqueSelector, TechniqueTraining, assign_technique,
};

use crate::career::Member;

/// The training the technique screen shows for a member. The arena's
/// skill screen sets neither the truncate nor the weighted flag (guess:
/// those belong to the story's character screen).
pub fn training_of(member: &Member) -> TechniqueTraining {
    let mut training = TechniqueTraining {
        pool: member.xp.min(i32::MAX as u32) as i32,
        ..TechniqueTraining::default()
    };
    for (skill, record) in training.skills.iter_mut().enumerate() {
        *record = SkillRecord {
            points: member.skill_xp[skill],
            techniques: member.techniques[skill],
        };
    }
    training
}

/// Writes the selector's training back into the member: the six skills,
/// their levels and the pool.
pub fn save_to(member: &mut Member, training: &TechniqueTraining) {
    for skill in 0..SKILLS {
        member.skill_xp[skill] = training.skills[skill].points;
        member.techniques[skill] = training.skills[skill].techniques;
        member.skills[skill] = u32::from(training.skills[skill].points / 5_000);
    }
    member.xp = training.pool.max(0) as u32;
}

/// The selector for skill `skill` (1…6) and slot `slot` (1…5) of the
/// skill screen, with nothing picked from it yet.
pub fn selector_for(skill: u16, slot: u16) -> TechniqueSelector {
    TechniqueSelector {
        skill,
        slot,
        last: 0,
    }
}

/// Picks `technique` (or forgets, when negative) into a member's slot and
/// stores the result.
pub fn pick(
    member: &mut Member,
    selector: &mut TechniqueSelector,
    technique: i32,
) -> TechniqueChange {
    let mut training = training_of(member);
    let change = assign_technique(&mut training, selector, technique, false);
    if change.changed {
        save_to(member, &training);
    }
    change
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::{Company, Profession};

    fn member() -> Member {
        let mut company = Company::new("Ash");
        company.hire("Bran", Profession::Combatant).unwrap();
        company.roster.remove(0)
    }

    #[test]
    fn a_member_training_round_trips() {
        let mut member = member();
        member.xp = 12_345;
        member.skill_xp = [5_000, 2_500, 0, 0, 0, 0];
        member.techniques[0] = [3, 4, 0, 0, 0, 0];
        let training = training_of(&member);
        assert_eq!(training.pool, 12_345);
        assert_eq!(training.skills[0].points, 5_000);
        assert_eq!(training.skills[0].techniques[..2], [3, 4]);
        assert_eq!(training.skills[1].points, 2_500);
        save_to(&mut member, &training);
        assert_eq!(member.skill_xp, [5_000, 2_500, 0, 0, 0, 0]);
        assert_eq!(member.skills, [1, 0, 0, 0, 0, 0]);
        assert_eq!(member.techniques[0], [3, 4, 0, 0, 0, 0]);
        assert_eq!(member.xp, 12_345);
        // A negative pool is stored as zero.
        let mut training = training;
        training.pool = -5;
        save_to(&mut member, &training);
        assert_eq!(member.xp, 0);
    }

    #[test]
    fn a_pick_moves_the_unspent_experience_into_the_skill() {
        let mut member = member();
        member.xp = 9_000;
        member.skill_xp = [5_000, 0, 0, 0, 0, 0];
        member.techniques[0] = [3, 0, 0, 0, 0, 0];
        let mut selector = selector_for(1, 2);
        let change = pick(&mut member, &mut selector, 5);
        assert!(change.changed);
        // 9 000 − 5 000 unspent: 4 000 into skill 1, whose slot 2 is now
        // in training.
        assert_eq!(member.techniques[0][..2], [3, 5]);
        assert_eq!(member.skill_xp[0], 9_000);
        assert_eq!(selector.last, 5);
        // Forgetting it clears the last filled slot: one level held.
        pick(&mut member, &mut selector, -1);
        assert_eq!(member.techniques[0][..2], [3, 0]);
        assert_eq!(member.skill_xp[0], 5_000);
        assert_eq!(member.skills[0], 1);
    }

    /// The selector's forget and its technique in training are the
    /// character record's own unlearning and surplus slot: both follow the
    /// same routines, so they agree on every record.
    #[test]
    fn the_selector_agrees_with_the_live_record() {
        use exa_character::record::CharacterData;
        let records: [[u8; 6]; 5] = [
            [1, 2, 3, 4, 0, 0],
            [2, 3, 4, 1, 0, 0],
            [1, 2, 0, 5, 0, 0],
            [4, 3, 2, 1, 8, 0],
            [0, 0, 0, 0, 0, 0],
        ];
        for skill in 1..=6u16 {
            for techniques in records {
                for points in [0u16, 7_500, 15_000, 20_000] {
                    for slot in 1..=5u16 {
                        let mut training = TechniqueTraining::default();
                        let index = usize::from(skill) - 1;
                        training.skills[index] = SkillRecord { points, techniques };
                        let mut data = CharacterData::new();
                        data.record.skills = training.skills;
                        assert_eq!(
                            training
                                .in_training()
                                .map(|(s, l)| (usize::from(s), u32::from(l))),
                            data.surplus_chosen_level()
                        );
                        training.forget(skill, slot);
                        data.unlearn_level(usize::from(skill), u32::from(slot));
                        assert_eq!(
                            training.skills, data.record.skills,
                            "{skill} {techniques:?} {slot}"
                        );
                    }
                }
            }
        }
    }
}
