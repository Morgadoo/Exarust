//! The fighting estimate: what the experience system says a character is
//! worth in a fight.
//!
//! One skill level is 5000 points. The estimate is `2 + 0.4·levels + 0.02`
//! times the combat skill, with a held weapon adding `0.02` times the sum
//! of its impact, thrust, slash, crush and pierce hundredths times its
//! reach. A body scale above 1.3 adds 2; otherwise the estimate is
//! quartered.
//!
//! The sums keep the original's precision and order: the level part and
//! the combat-skill part are added in double precision, the weapon bytes
//! are summed in single precision, and the weapon term joins the estimate
//! in double precision. The share of each weapon byte is a hundredth: the
//! original divides by 100 rather than using a precomputed factor.

use exa_formats::characters::{SKILLS, Skill};
use exa_formats::weapon_stats::{StatBytes, at};

/// Skill experience in one level.
pub const LEVEL_STEP: u16 = 5000;

/// The level a skill's points buy.
pub fn skill_level(points: u16) -> u32 {
    u32::from(points) / u32::from(LEVEL_STEP)
}

/// The six skills' levels summed.
pub fn total_levels(skills: &[Skill; SKILLS]) -> i32 {
    skills
        .iter()
        .map(|skill| skill_level(skill.points) as i32)
        .sum()
}

/// The level and combat-skill part of the estimate, before any weapon and
/// the body's scale: `(levels·0.4 + 2) + combat_skill·0.02`, the first
/// term in single precision and the sum in double, cut back to a float.
pub fn fighting_base(skills: &[Skill; SKILLS], combat_skill: f32) -> f32 {
    let levels = total_levels(skills);
    let level_part = (levels as f32) * 0.4 + 2.0;
    (f64::from(combat_skill) * 0.02 + f64::from(level_part)) as f32
}

/// Add the held weapon and the body scale to a base estimate.
///
/// `weapon` is the weapon's stat bytes and reach in cm. The bytes are
/// summed in single precision, each a hundredth, then the sum times the
/// reach joins the base in double precision and is cut back to a float.
pub fn fighting_estimate_with_base(
    base: f32,
    weapon: Option<(&StatBytes, f32)>,
    scale: f32,
) -> f32 {
    let mut estimate = base;
    if let Some((stats, reach)) = weapon {
        let mut sum = 0.0f32;
        for byte in [at::IMPACT, at::SLASH, at::CRUSH, at::PIERCE, at::THRUST] {
            let part = f64::from(stats.get(byte)) * 0.01;
            sum = if sum == 0.0 {
                part as f32
            } else {
                (part + f64::from(sum)) as f32
            };
        }
        let weighted = sum * reach;
        estimate = (f64::from(weighted) * 0.02 + f64::from(estimate)) as f32;
    }
    if f64::from(scale) > 1.3 {
        estimate + 2.0
    } else {
        estimate * 0.25
    }
}

/// A combatant's fighting estimate: the record's skills and combat skill,
/// its held weapon's stat bytes times its reach, and its body scale.
pub fn fighting_estimate(
    skills: &[Skill; SKILLS],
    combat_skill: f32,
    weapon: Option<(&StatBytes, f32)>,
    scale: f32,
) -> f32 {
    fighting_estimate_with_base(fighting_base(skills, combat_skill), weapon, scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skills_with(points: [u16; SKILLS]) -> [Skill; SKILLS] {
        let mut skills = [Skill::default(); SKILLS];
        for (skill, points) in skills.iter_mut().zip(points) {
            skill.points = points;
        }
        skills
    }

    /// Three levels and combat skill 50: 2 + 1.2 + 1 = 4.2, quartered 1.05.
    /// With the weapon's five hundredths at reach 120 and scale 1.5:
    /// 4.2 + 6.408 + 2.
    #[test]
    fn the_fighting_estimate_follows_the_skills_the_weapon_and_the_scale() {
        let skills = skills_with([5000, 10_000, 0, 0, 0, 0]);
        let alone = fighting_estimate(&skills, 50.0, None, 1.0);
        assert_eq!(alone.to_bits(), 0x3f86_6666);
        let mut stats = StatBytes::default();
        for (byte, value) in [
            (at::IMPACT, 127),
            (at::SLASH, 50),
            (at::CRUSH, 20),
            (at::PIERCE, 40),
            (at::THRUST, 30),
        ] {
            stats.set(byte, value);
        }
        let armed = fighting_estimate(&skills, 50.0, Some((&stats, 120.0)), 1.5);
        assert_eq!(armed.to_bits(), 0x4149_ba5f);
        // The base is the level part and the combat skill in double, cut to
        // a float: 3 levels, combat skill 50.
        assert_eq!(fighting_base(&skills, 50.0).to_bits(), 4.2f32.to_bits());
        // A scale of exactly 1.3 quarters the estimate, and so does less.
        assert_eq!(
            fighting_estimate_with_base(4.2, None, 1.3).to_bits(),
            1.05f32.to_bits()
        );
    }

    #[test]
    fn a_skill_level_is_five_thousand_points() {
        assert_eq!(skill_level(0), 0);
        assert_eq!(skill_level(4999), 0);
        assert_eq!(skill_level(5000), 1);
        assert_eq!(skill_level(25_000), 5);
        assert_eq!(total_levels(&skills_with([5000, 5000, 5000, 0, 0, 0])), 3);
    }
}
