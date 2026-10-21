//! Who a career match spawns, and where each side stands.
//!
//! Every match is laid out around the arena's spawn spot. A duel, a
//! challenge, an elimination, a reserve match and pugilism put one
//! opponent 2 m before the spot (and 5 cm up) facing +z and the company's
//! fighter 2 m behind it facing −z. Team matches stand on a ring of 3.5 m:
//! a fraction of a turn picks the direction `(sin, 0, cos)` with the
//! engine's sine and cosine, the fighter stands 3.5 m against it and faces
//! along it, so every fighter looks at the spot. Doubles: opponents at
//! ∓0.05 and the company at 0.55 and 0.45. Valiance: two opponents a level
//! and a tier lower at ∓1/15, one fighter at 0.5. Valour: three opponents two
//! lower at −1/12, 0, 1/12. Captain: the same with the middle one at the
//! match's own level. Skirmish: three at the match's level, the company at
//! 7/12, 0.5 and 5/12. The Beast: the beast at the opponent's spot, the
//! company on the same three ring places. A fray or a brawl puts six
//! fighters every sixth of a turn, the company's fighter fourth, each
//! opponent on a team of its own. Pugilism's opponent has no gear tier.
//!
//! Evidence: the retail arena's eleven template setups, the ring placement,
//! the fixed placement templates and the next-round routine read from
//! `Exanima.exe` (`context/re/arena-layout.md`).

use crate::thaum::engine::sin_cos;

/// The opponent's spot from the arena's spawn spot (cm): 2 m ahead, 5 cm up.
pub const OPPONENT_SPOT: [f32; 3] = [0.0, 5.0, -200.0];
/// The company's spot from the spawn spot (cm).
pub const OUR_SPOT: [f32; 3] = [0.0, 0.0, 200.0];
/// The ring's radius (cm), the factor the direction is scaled by.
pub const RING_SCALE: f32 = -350.0;
/// A full turn in single precision, as the setups multiply it.
pub const TURN: f32 = f32::from_bits(0x40c9_0fdb);
/// The team every ordinary opponent fights on.
pub const OPPONENT_TEAM: u32 = 2;
/// An opponent's AI flags: hostile to the company (0x12), or in a free for
/// all (0x10).
pub const HOSTILE: u32 = 0x12;
pub const FREE_FOR_ALL: u32 = 0x10;

/// Who stands in a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fighter {
    /// An opponent of this level and gear tier (−1: no gear).
    Opponent {
        level: i32,
        tier: i32,
        team: u32,
        flags: u32,
    },
    /// The Beast.
    Beast { level: i32, tier: i32 },
    /// The company's entrant at this place in the entry list.
    Ours(usize),
}

/// Where a slot stands (cm, the arena's frame) and which way it faces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub position: [f32; 3],
    pub forward: [f32; 3],
}

/// One slot of a match's layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub who: Fighter,
    pub place: Place,
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// The opponent's fixed place: the spawn spot plus [`OPPONENT_SPOT`],
/// facing +z.
pub fn opponent_place(spawn: [f32; 3]) -> Place {
    Place {
        position: add(OPPONENT_SPOT, spawn),
        forward: [0.0, 0.0, 1.0],
    }
}

/// The company's fixed place: the spawn spot plus [`OUR_SPOT`], facing −z.
pub fn our_place(spawn: [f32; 3]) -> Place {
    Place {
        position: add(spawn, OUR_SPOT),
        forward: [0.0, 0.0, -1.0],
    }
}

/// A ring place from a direction angle: the direction is the engine's
/// `(sin, 0, cos)` of it, the place the spawn spot plus the direction
/// times −350, facing along the direction.
pub fn ring_at(spawn: [f32; 3], angle: f32) -> Place {
    let (sin, cos) = sin_cos(angle);
    let direction = [sin, 0.0, cos];
    Place {
        position: [
            spawn[0] + direction[0] * RING_SCALE,
            spawn[1] + direction[1] * RING_SCALE,
            spawn[2] + direction[2] * RING_SCALE,
        ],
        forward: direction,
    }
}

/// A ring place at a fraction of a turn (the turn times the fraction).
pub fn ring_place(spawn: [f32; 3], fraction: f32) -> Place {
    ring_at(spawn, TURN * fraction)
}

/// A fray's place `index` of six: the index times the turn, times a sixth.
pub fn fray_place(spawn: [f32; 3], index: i32) -> Place {
    ring_at(spawn, index as f32 * TURN * f32::from_bits(0x3e2a_aaab))
}

fn opponent(level: i32, tier: i32) -> Fighter {
    Fighter::Opponent {
        level,
        tier,
        team: OPPONENT_TEAM,
        flags: HOSTILE,
    }
}

const FIFTEENTH: u32 = 0x3d88_8889;
const TWELFTH: u32 = 0x3daa_aaab;
const SEVEN_TWELFTHS: u32 = 0x3f15_5555;
const FIVE_TWELFTHS: u32 = 0x3ed5_5555;

fn at(fraction_bits: u32) -> f32 {
    f32::from_bits(fraction_bits)
}

/// The slots a match of `template` spawns at its `level` and gear `tier`
/// around `spawn`, opponents first in spawn order, then the company's
/// entrants. An unknown template spawns nobody.
pub fn layout(template: i32, level: i32, tier: i32, spawn: [f32; 3]) -> Vec<Slot> {
    let ring = |fraction: f32| ring_place(spawn, fraction);
    let ours = |index: usize, place: Place| Slot {
        who: Fighter::Ours(index),
        place,
    };
    let foe = |level, tier, place| Slot {
        who: opponent(level, tier),
        place,
    };
    let lower = |by: i32| (level.wrapping_sub(by), tier.wrapping_sub(by));
    match template {
        // Duel, Challenger, Elimination, Reserve.
        1 | 8 | 7 | 5 => vec![
            foe(level, tier, opponent_place(spawn)),
            ours(0, our_place(spawn)),
        ],
        // Pugilism: no gear.
        6 => vec![
            foe(level, -1, opponent_place(spawn)),
            ours(0, our_place(spawn)),
        ],
        // Doubles.
        2 => vec![
            foe(level, tier, ring(-0.05)),
            foe(level, tier, ring(0.05)),
            ours(0, ring(f32::from_bits(0x3f0c_cccd))),
            ours(1, ring(f32::from_bits(0x3ee6_6666))),
        ],
        // Valiance: two a rank lower.
        13 => {
            let (l, t) = lower(1);
            vec![
                foe(l, t, ring(-at(FIFTEENTH))),
                foe(l, t, ring(at(FIFTEENTH))),
                ours(0, ring(0.5)),
            ]
        }
        // Valour: three two ranks lower.
        14 => {
            let (l, t) = lower(2);
            vec![
                foe(l, t, ring(-at(TWELFTH))),
                foe(l, t, ring(0.0)),
                foe(l, t, ring(at(TWELFTH))),
                ours(0, ring(0.5)),
            ]
        }
        // Captain: the middle opponent at the match's level.
        9 => {
            let (l, t) = lower(2);
            vec![
                foe(l, t, ring(-at(TWELFTH))),
                foe(level, tier, ring(0.0)),
                foe(l, t, ring(at(TWELFTH))),
                ours(0, ring(0.5)),
                ours(1, ring(at(SEVEN_TWELFTHS))),
                ours(2, ring(at(FIVE_TWELFTHS))),
            ]
        }
        // Skirmish.
        3 => vec![
            foe(level, tier, ring(-at(TWELFTH))),
            foe(level, tier, ring(0.0)),
            foe(level, tier, ring(at(TWELFTH))),
            ours(0, ring(at(SEVEN_TWELFTHS))),
            ours(1, ring(0.5)),
            ours(2, ring(at(FIVE_TWELFTHS))),
        ],
        // The Beast.
        11 => vec![
            Slot {
                who: Fighter::Beast { level, tier },
                place: opponent_place(spawn),
            },
            ours(0, ring(at(SEVEN_TWELFTHS))),
            ours(1, ring(0.5)),
            ours(2, ring(at(FIVE_TWELFTHS))),
        ],
        // Fray and Brawl: six places, the company fourth.
        4 | 12 => (0..6)
            .map(|index| {
                let place = fray_place(spawn, index);
                if index == 3 {
                    ours(0, place)
                } else {
                    Slot {
                        who: Fighter::Opponent {
                            level,
                            tier,
                            team: 1 << (index + 3),
                            flags: FREE_FOR_ALL,
                        },
                        place,
                    }
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Who fights the next round of a multi-round match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextRound {
    /// No round is lost yet, or the company's fighter still stands: it
    /// fights on and a fresh opponent of the match's level and tier comes.
    FreshOpponent,
    /// The last round was lost and the fighter is dead: the opponent stays
    /// and the entrant at the place of the losses comes in.
    Reserve(usize),
}

/// The next round's change.
pub fn next_round(losses: i32, fighter_dead: bool) -> NextRound {
    if losses < 1 || !fighter_dead {
        NextRound::FreshOpponent
    } else {
        NextRound::Reserve(losses as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(place: &Place) -> ([u32; 3], [u32; 3]) {
        (
            place.position.map(f32::to_bits),
            place.forward.map(f32::to_bits),
        )
    }

    #[test]
    fn a_duel_faces_off_across_the_spot() {
        let spawn = [900.0, 0.0, 100.0];
        let slots = layout(1, 3, 2, spawn);
        assert_eq!(slots.len(), 2);
        assert_eq!(
            slots[0].who,
            Fighter::Opponent {
                level: 3,
                tier: 2,
                team: 2,
                flags: 0x12
            }
        );
        assert_eq!(slots[0].place.position, [900.0, 5.0, -100.0]);
        assert_eq!(slots[0].place.forward, [0.0, 0.0, 1.0]);
        assert_eq!(slots[1].who, Fighter::Ours(0));
        assert_eq!(slots[1].place.position, [900.0, 0.0, 300.0]);
        assert_eq!(slots[1].place.forward, [0.0, 0.0, -1.0]);
        // Pugilism takes no gear.
        let fists = layout(6, 3, 2, spawn);
        assert!(matches!(fists[0].who, Fighter::Opponent { tier: -1, .. }));
        assert!(layout(99, 3, 2, spawn).is_empty());
    }

    #[test]
    fn the_ring_puts_a_half_turn_behind_the_spot() {
        let half = ring_place([0.0; 3], 0.5);
        assert_eq!(
            bits(&half).0[0],
            (-350.0f32 * sin_cos(TURN * 0.5).0).to_bits()
        );
        assert!((half.position[2] - 350.0).abs() < 1e-3);
        assert!((half.forward[2] + 1.0).abs() < 1e-6);
        let zero = ring_place([10.0, 20.0, 30.0], 0.0);
        assert_eq!(zero.position, [10.0, 20.0, -320.0]);
        assert_eq!(zero.forward, [0.0, 0.0, 1.0]);
        // The doubles opponents stand mirrored.
        let left = ring_place([0.0; 3], -0.05);
        let right = ring_place([0.0; 3], 0.05);
        assert_eq!(left.position[0], -right.position[0]);
        assert_eq!(left.position[2], right.position[2]);
        assert!((right.position[0] + 108.155).abs() < 1e-2);
    }

    #[test]
    fn team_matches_lower_their_opponents() {
        let spawn = [0.0; 3];
        let valiance = layout(13, 4, 3, spawn);
        assert_eq!(valiance.len(), 3);
        assert!(matches!(
            valiance[0].who,
            Fighter::Opponent {
                level: 3,
                tier: 2,
                ..
            }
        ));
        let valour = layout(14, 4, 3, spawn);
        assert_eq!(
            valour
                .iter()
                .filter(|s| !matches!(s.who, Fighter::Ours(_)))
                .count(),
            3
        );
        assert!(matches!(
            valour[1].who,
            Fighter::Opponent {
                level: 2,
                tier: 1,
                ..
            }
        ));
        let captain = layout(9, 4, 3, spawn);
        assert!(matches!(captain[0].who, Fighter::Opponent { level: 2, .. }));
        assert!(matches!(
            captain[1].who,
            Fighter::Opponent {
                level: 4,
                tier: 3,
                ..
            }
        ));
        assert_eq!(captain[4].who, Fighter::Ours(1));
        assert_eq!(
            captain[4].place,
            ring_place(spawn, f32::from_bits(0x3f15_5555))
        );
        let skirmish = layout(3, 4, 3, spawn);
        assert_eq!(
            skirmish[3].place,
            ring_place(spawn, f32::from_bits(0x3f15_5555))
        );
        assert_eq!(skirmish[4].place, ring_place(spawn, 0.5));
        let beast = layout(11, 5, 5, spawn);
        assert_eq!(beast[0].who, Fighter::Beast { level: 5, tier: 5 });
        assert_eq!(beast[0].place, opponent_place(spawn));
        let doubles = layout(2, 1, 1, spawn);
        assert_eq!(
            doubles[2].place,
            ring_place(spawn, f32::from_bits(0x3f0c_cccd))
        );
    }

    #[test]
    fn a_fray_spreads_six_teams() {
        let slots = layout(4, 2, 2, [0.0; 3]);
        assert_eq!(slots.len(), 6);
        assert_eq!(slots[3].who, Fighter::Ours(0));
        let teams: Vec<u32> = slots
            .iter()
            .filter_map(|s| match s.who {
                Fighter::Opponent { team, flags, .. } => {
                    assert_eq!(flags, FREE_FOR_ALL);
                    Some(team)
                }
                _ => None,
            })
            .collect();
        assert_eq!(teams, vec![8, 16, 32, 128, 256]);
        assert_eq!(slots[0].place.position, [0.0, 0.0, -350.0]);
        assert_eq!(layout(12, 2, 2, [0.0; 3]), slots);
    }

    #[test]
    fn the_next_round_brings_a_fresh_opponent_or_a_reserve() {
        assert_eq!(next_round(0, true), NextRound::FreshOpponent);
        assert_eq!(next_round(1, false), NextRound::FreshOpponent);
        assert_eq!(next_round(2, true), NextRound::Reserve(2));
    }
}
