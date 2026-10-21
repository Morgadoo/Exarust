//! The technique selector's bridge to a roster member.
//!
//! The original edits the character's six skill records (`+0xc0`, six
//! 16-bit experience words each) and its five assignment bytes. Exarust's
//! [`Member`] keeps one experience word per skill and six technique ids, so
//! the selector's first experience word carries the skill's total and the
//! member's `xp` carries the unspent pool. The level is still experience
//! over 5 000.

use exa_ui::{ASSIGNED, LEVEL_XP, ROWS, SkillTraining, TechniqueSelector, TechniqueTraining};

use crate::career::Member;

/// The training the technique screen shows for a member.
pub fn training_of(member: &Member) -> TechniqueTraining {
    let mut training = TechniqueTraining {
        pool: member.xp as i32,
        ..TechniqueTraining::default()
    };
    for skill in 0..ROWS.min(member.techniques.len()) {
        training.skills[skill].xp[0] = member.skill_xp[skill];
        for slot in 0..ASSIGNED {
            training.skills[skill].assigned[slot] = member.techniques[skill][slot];
        }
    }
    training
}

/// Writes the selector's training back into the member.
pub fn save_to(member: &mut Member, training: &TechniqueTraining) {
    for skill in 0..ROWS.min(member.techniques.len()) {
        member.skill_xp[skill] = training.skills[skill].xp[0];
        for slot in 0..ASSIGNED {
            member.techniques[skill][slot] = training.skills[skill].assigned[slot];
        }
    }
    member.xp = training.pool.max(0) as u32;
}

/// The selector for one skill and one learned slot.
pub fn selector_for(member: &Member, skill: usize, slot: usize) -> TechniqueSelector {
    let skill = skill.min(ROWS.min(member.techniques.len()).saturating_sub(1));
    let slot = slot.clamp(1, ASSIGNED);
    let picked = member.techniques[skill][slot - 1] as i32;
    TechniqueSelector {
        index: if picked == 0 { -1 } else { picked },
        slot,
        group: skill,
        ..TechniqueSelector::default()
    }
}

/// A skill's level from its stored experience.
pub fn level(experience: u16) -> u16 {
    experience / LEVEL_XP
}

/// The experience a technique needs for the given level.
pub fn level_experience(level: u16) -> u16 {
    level.saturating_mul(LEVEL_XP)
}

/// One skill's training after an assignment, for the member's use.
pub fn skill_after(training: &TechniqueTraining, skill: usize) -> SkillTraining {
    training.skills[skill.min(ROWS - 1)]
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
        assert_eq!(training.skills[0].xp[0], 5_000);
        assert_eq!(training.skills[0].assigned[..2], [3, 4]);
        assert_eq!(training.skills[1].xp[0], 2_500);
        save_to(&mut member, &training);
        assert_eq!(member.skill_xp, [5_000, 2_500, 0, 0, 0, 0]);
        assert_eq!(member.techniques[0][..5], [3, 4, 0, 0, 0]);
        assert_eq!(member.xp, 12_345);
        // A negative pool is stored as zero.
        let mut training = training;
        training.pool = -5;
        save_to(&mut member, &training);
        assert_eq!(member.xp, 0);
    }

    #[test]
    fn a_selector_reads_the_slot_and_index() {
        let mut member = member();
        member.techniques[2] = [0, 7, 0, 0, 0, 0];
        let selector = selector_for(&member, 2, 2);
        assert_eq!(selector.group, 2);
        assert_eq!(selector.slot, 2);
        assert_eq!(selector.index, 7);
        let empty = selector_for(&member, 2, 1);
        assert_eq!(empty.index, -1);
        assert_eq!(selector_for(&member, 9, 9).slot, 5);
        assert_eq!(selector_for(&member, 9, 9).group, 5);
        assert_eq!(level(12_499), 2);
        assert_eq!(level_experience(3), 15_000);
        let skill = skill_after(&training_of(&member), 2);
        assert_eq!(skill.assigned[1], 7);
    }
}
