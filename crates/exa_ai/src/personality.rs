//! What the brain takes from a character's record when it is made, and how
//! it is set up when attached to the body.
//!
//! - **Personality.** The skill share is the combat skill over 100, capped
//!   at 1. With t1 and t2 the record's second and third traits: the confidence
//!   c = t1/100, the restraint r = −t2/100, the courage (c + 1)/2, the
//!   eagerness ((3 − r) + (3 + c))/8, and the blend `(1 − h) + h·(c + 2)·0.333`
//!   with h = (1 − feral)² (1 for a marked body). A jitter is the square of a
//!   draw spread over −1..1. The arousal starts at 10, the state at the
//!   wait, the roam at its first phase.
//! - **Attaching.** Without the race word's AI flag no contact sense is
//!   installed and the state is the wait. With it, the sense is the
//!   standard one for race class `0x12xx` and the ally sense otherwise; a dead
//!   character gets the effect at 0 and nothing else; a statue race stands
//!   sentry in the guard motion (`0x19`), effect at 0, marked as a statue;
//!   any other gets the effect at 1. Then the roles are applied, which
//!   choose the state.
//!
//! Evidence: the original's AI constructor and its attach (init) method.
//! The names of the traits' shares are Exarust's [guess: what the traits
//! mean]; the effect the attach sets is not identified [?].

/// The race word's AI flag.
pub const AI_FLAG: u32 = 0x1000;
/// The race word's statue flag.
pub const STATUE_FLAG: u32 = 0x800_0000;
/// The race class that keeps the standard sense; every other AI race
/// uses the ally sense.
pub const ALLY_CLASS: u32 = 0x1200;
/// The guard motion a statue stands in.
pub const GUARD_MOTION: i32 = 0x19;
/// The arousal a new brain starts with.
pub const AROUSAL: f32 = 10.0;

/// What the brain derives from the record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Personality {
    pub skill: f32,
    pub confidence: f32,
    pub restraint: f32,
    pub courage: f32,
    pub eagerness: f32,
    pub blend: f32,
    pub jitter: f32,
    pub arousal: f32,
    /// A share the construct mark zeroes (1 otherwise) [?].
    pub openness: f32,
}

/// The engine's draw in 0..1: the generator stepped, over 2³².
pub fn unit_draw(roll: &mut u32) -> f32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    (f64::from(*roll) * (1.0 / 4_294_967_296.0)) as f32
}

/// The personality of a record with `combat_skill`, `traits` and `feral`,
/// `marked` for a body with the mark that fixes the blend.
pub fn personality(
    combat_skill: f32,
    traits: [f32; 4],
    feral: f32,
    marked: bool,
    roll: &mut u32,
) -> Personality {
    // The shares over 100 are formed in double precision from the f64 0.01.
    let hundredth = |v: f32| (f64::from(v) * 0.01) as f32;
    let mut skill = hundredth(combat_skill);
    if 1.0 < skill {
        skill = 1.0;
    }
    let confidence = hundredth(traits[1]);
    let restraint = hundredth(-traits[2]);
    let courage = (confidence + 1.0) * 0.5;
    let blend = if marked {
        1.0
    } else {
        let h = 1.0 - feral;
        let h = h * h;
        // (1 − h) + h·((c + 2)·0.333), the product and sum in double.
        (f64::from(1.0 - h) + f64::from(h) * (f64::from(confidence + 2.0) * 0.333)) as f32
    };
    // The original sums (3 − r) + (3 + c), in that order.
    let eagerness = ((3.0 - restraint) + (3.0 + confidence)) * 0.125;
    let spread = (unit_draw(roll) - 0.5) * 2.0;
    Personality {
        skill,
        confidence,
        restraint,
        courage,
        eagerness,
        blend,
        jitter: spread * spread,
        arousal: AROUSAL,
        openness: if marked { 0.0 } else { 1.0 },
    }
}

/// Which contact sense the brain uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sense {
    /// The standard per-contact sense of race class `0x12xx`: it weighs the
    /// observed character's contacts as the threat estimate and takes the
    /// leader for a friend.
    #[default]
    Standard,
    /// The ally-aware sense of the other AI races: a fellow AI is an ally
    /// and each contact's threat is the largest threat to the allies
    /// around the observer.
    Ally,
}

/// How attaching sets the brain up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attached {
    pub sense: Option<Sense>,
    /// It stands sentry as a statue, in the guard motion.
    pub statue: bool,
    /// The level of the effect attaching sets, if any [?].
    pub effect: Option<f32>,
    /// Its roles are applied next.
    pub applies_roles: bool,
}

/// Attaching a brain to a body of race word `race_word`.
pub fn attach(race_word: u32, dead: bool) -> Attached {
    if race_word & AI_FLAG == 0 {
        return Attached {
            sense: None,
            statue: false,
            effect: None,
            applies_roles: true,
        };
    }
    // Race class `0x12xx` gets the standard sense; the other AI races the
    // ally-aware one.
    let sense = if race_word & 0xFF00 == ALLY_CLASS {
        Sense::Standard
    } else {
        Sense::Ally
    };
    if dead {
        return Attached {
            sense: Some(sense),
            statue: false,
            effect: Some(0.0),
            applies_roles: false,
        };
    }
    let statue = race_word & STATUE_FLAG != 0;
    Attached {
        sense: Some(sense),
        statue,
        effect: Some(if statue { 0.0 } else { 1.0 }),
        applies_roles: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skill 150 caps at 1; t1 20 and t2 −30 give confidence 0.2, restraint
    /// 0.3, courage 0.6, eagerness (2.7 + 3.2)/8; feral 0.5 blends
    /// 0.75 + 0.25·2.2·0.333; the first draw of a generator at 0 is 2⁻³²,
    /// so the jitter is about 1.
    #[test]
    fn the_record_gives_the_personality() {
        let mut roll = 0;
        let p = personality(150.0, [0.0, 20.0, -30.0, 0.0], 0.5, false, &mut roll);
        assert_eq!(p.skill, 1.0);
        assert!((p.confidence - 0.2).abs() < 1e-6);
        assert!((p.restraint - 0.3).abs() < 1e-6);
        assert!((p.courage - 0.6).abs() < 1e-6);
        assert!((p.eagerness - 5.9 / 8.0).abs() < 1e-6);
        assert!((p.blend - (0.75 + 0.25 * 2.2 * 0.333)).abs() < 1e-6);
        assert!((p.jitter - 1.0).abs() < 1e-6);
        assert_eq!((p.arousal, p.openness), (10.0, 1.0));
        assert_eq!(roll, 1);
        // The hundredth is taken in double: 5 gives the f32 nearest 0.05
        // (bits 0x3d4ccccd), where an f32 product gives 0x3d4ccccc.
        let p = personality(5.0, [0.0, 5.0, -5.0, 0.0], 0.0, false, &mut roll);
        assert_eq!(p.skill.to_bits(), 0x3d4c_cccd);
        assert_eq!(p.confidence.to_bits(), 0x3d4c_cccd);
        assert_eq!(p.restraint.to_bits(), 0x3d4c_cccd);
        let marked = personality(40.0, [0.0; 4], 1.0, true, &mut roll);
        assert!((marked.skill - 0.4).abs() < 1e-6);
        assert_eq!((marked.blend, marked.openness), (1.0, 0.0));
        let human = personality(0.0, [0.0; 4], 0.0, false, &mut roll);
        assert!((human.blend - 2.0 * 0.333).abs() < 1e-6);
    }

    /// No AI flag waits; race class `0x12xx` gets the standard sense and
    /// the other AI races the ally sense; a dead body stops at the effect
    /// 0; a statue stands sentry with the effect 0.
    #[test]
    fn attaching_picks_the_sense_and_the_statue() {
        let none = attach(0x0100, false);
        assert_eq!((none.sense, none.applies_roles), (None, true));
        let class = attach(AI_FLAG | 0x1200, false);
        assert_eq!(class.sense, Some(Sense::Standard));
        assert_eq!(class.effect, Some(1.0));
        let other = attach(AI_FLAG | 0x0400, false);
        assert_eq!(other.sense, Some(Sense::Ally));
        let dead = attach(AI_FLAG, true);
        assert_eq!((dead.effect, dead.applies_roles), (Some(0.0), false));
        assert_eq!(dead.sense, Some(Sense::Ally));
        let statue = attach(AI_FLAG | STATUE_FLAG | 0x0400, false);
        assert!(statue.statue);
        assert_eq!(
            (statue.sense, statue.effect),
            (Some(Sense::Ally), Some(0.0))
        );
    }
}
