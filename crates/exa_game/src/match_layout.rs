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
//! A fresh tournament match then creates those bodies. A side stops at the
//! first body the generator cannot create and reports the failure; the
//! bodies that were created on that side are given familiarity 100 toward
//! each other, both ways, and both of their AI clocks wait 3000 ms. Fewer
//! than three formed sides are hostile; three or more are a free-for-all.
//! The side whose id is the player's is stood from the company list, and
//! the player's own entry is not spawned again. A rematch, which drops
//! bodies in the dead mode and repositions the rest, is a separate branch.
//!
//! Evidence: the retail arena's eleven template setups, the ring placement,
//! the fixed placement templates, the next-round routine and the fresh
//! match generator, read from `Exanima.exe` 0.9.5 (`context/re/arena-layout.md`).

use crate::arena_spawn::{
    GenericPlan, HirePlan, MANAGER_ROLE, SPAWN_DAMAGE, SPAWN_HEALTH, generic_plan, hire_plan,
};
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

/// The double a bracket place scales its turn by (not quite a sixth).
pub const BRACKET_SIXTH: f64 = 0.166_666_6;

/// A bracket place: the turn times `(3 − a) × 0.1666666` (that product in
/// double, stored as single) gives the direction, and the fighter stands
/// `radius` against it, facing along it.
pub fn bracket_place(spawn: [f32; 3], a: f32, radius: f32) -> Place {
    let turns = ((f64::from(3.0f32 - a)) * BRACKET_SIXTH) as f32;
    let (sin, cos) = sin_cos(TURN * turns);
    let direction = [sin, 0.0, cos];
    let scale = -radius;
    Place {
        position: [
            spawn[0] + direction[0] * scale,
            spawn[1] + direction[1] * scale,
            spawn[2] + direction[2] * scale,
        ],
        forward: direction,
    }
}

/// One team of a tournament event.
#[derive(Clone, Debug, PartialEq)]
pub struct BracketTeam {
    /// The first entered slot's id: 1 is the player's team, 2 and up a
    /// company, others generic.
    pub team: i32,
    /// Each member's slot within its team and its place.
    pub members: Vec<(usize, Place)>,
}

/// Group a tournament event's entry slots into teams and place them.
///
/// From the first slot that holds an id, a team is the run of `per_team`
/// slots it falls in, its id the first held id, its members the slots with
/// a positive id; the next team starts after that run, until `teams` teams
/// are formed or the slots end. The ring's radius is
/// `teams × per_team × 50 + 100`, at most 350. Team `g` of `G` with `n`
/// members spreads them over `2 (n − 1)/(n + 1)` (in double, stored as
/// single) from `g/G × 6` less half the spread, a member every spread over
/// `n − 1`.
pub fn bracket_layout(
    slots: &[i32],
    per_team: i32,
    teams: i32,
    spawn: [f32; 3],
) -> Vec<BracketTeam> {
    let mut groups: Vec<(i32, Vec<usize>)> = Vec::new();
    let mut cursor = 0usize;
    if per_team > 0 {
        loop {
            while cursor < slots.len() && slots[cursor] == 0 {
                cursor += 1;
            }
            if cursor >= slots.len() {
                break;
            }
            let base = (cursor as i32 / per_team * per_team) as usize;
            let members = (0..per_team as usize)
                .filter(|k| slots.get(base + k).is_some_and(|id| *id > 0))
                .collect();
            groups.push((slots[cursor], members));
            cursor = base + per_team as usize;
            if groups.len() as i32 == teams {
                break;
            }
        }
    }
    let radius = (i64::from(teams) * i64::from(per_team) * 50 + 100).min(350) as f32;
    let count = groups.len() as f32;
    groups
        .into_iter()
        .enumerate()
        .map(|(index, (team, members))| {
            let n = members.len() as i64;
            let mut spread = ((n - 1) as f64 / (n + 1) as f64 * 2.0) as f32;
            let start = index as f32 / count * 6.0 - spread * 0.5;
            if n > 1 {
                spread /= (n - 1) as f32;
            }
            let members = members
                .into_iter()
                .enumerate()
                .map(|(order, slot)| {
                    let a = order as f32 * spread + start;
                    (slot, bracket_place(spawn, a, radius))
                })
                .collect();
            BracketTeam { team, members }
        })
        .collect()
}

/// A tournament opponent: a generic fighter of level 5 for a team id under
/// 2, else that company's fighter; hostile while at most two teams fight,
/// else every team for itself.
pub fn bracket_opponent(team: i32, teams: usize, tier: i32) -> Fighter {
    let flags = if teams <= 2 { HOSTILE } else { FREE_FOR_ALL };
    if team < 2 {
        Fighter::Opponent {
            level: 5,
            tier,
            team: team as u32,
            flags,
        }
    } else {
        Fighter::Opponent {
            level: -1,
            tier,
            team: team as u32,
            flags,
        }
    }
}

/// Both AI clocks of a body created for a fresh match wait this long.
pub const MATCH_ACT_GRACE_MS: u32 = 3_000;
/// Familiarity each generated teammate is given toward the others.
pub const TEAMMATE_FAMILIARITY: f32 = 100.0;
/// The report title when a match body cannot be created.
pub const MATCH_ERROR_TITLE: &str = "Error";
/// The report message when a match body cannot be created.
pub const MATCH_ERROR: &str = "Failed to generate match motile.";

/// The three condition words written on a character before its body is created.
///
/// A generated opponent writes −100, 100, 0. A company entrant writes
/// 100, 100, 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionWords {
    pub first: f32,
    pub second: f32,
    pub third: f32,
}

/// Condition words of a generated opponent.
pub const OPPONENT_CONDITION: ConditionWords = ConditionWords {
    first: SPAWN_DAMAGE,
    second: SPAWN_HEALTH,
    third: 0.0,
};

/// Condition words of a company entrant created for the player's side.
pub const COMPANY_CONDITION: ConditionWords = ConditionWords {
    first: SPAWN_HEALTH,
    second: SPAWN_HEALTH,
    third: 0.0,
};

/// The behaviour reset forced onto every generated opponent: mode 0,
/// parameter 20, applied even when that mode is already current.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BehaviourReset {
    pub mode: u32,
    pub parameter: u32,
    pub forced: bool,
}

/// The reset [`raise_fresh_match`] applies to a generated opponent.
pub const OPPONENT_BEHAVIOUR: BehaviourReset = BehaviourReset {
    mode: 0,
    parameter: 20,
    forced: true,
};

/// What a fresh match asks the generator to create for one opponent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpponentPlan {
    /// Team id under 2: level 5, dressed at the event's tier.
    Generic(GenericPlan),
    /// Team id of 2 or more: the hire-list profile. The profile's level
    /// byte is the event tier, not the team id.
    Hire(HirePlan),
}

/// One opponent the generator is asked to create.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpponentAttempt {
    pub slot: usize,
    pub team: i32,
    pub place: Place,
    pub plan: OpponentPlan,
}

/// A body a fresh match settled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RaisedBody {
    /// The player's own character, stood on its place and not spawned again.
    Player,
    /// Another company entrant. Role [`MANAGER_ROLE`], both AI clocks held.
    Company {
        condition: ConditionWords,
        role: u32,
        act_at_ms: u32,
    },
    /// A generated opponent. Its pending-action word is cleared.
    Opponent {
        plan: OpponentPlan,
        condition: ConditionWords,
        role: u32,
        act_at_ms: u32,
        behaviour: BehaviourReset,
        action_cleared: bool,
    },
}

/// One placed fighter of a fresh match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaisedFighter {
    pub slot: usize,
    pub team: i32,
    pub place: Place,
    pub body: RaisedBody,
}

/// Familiarity from one generated teammate toward another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TeammateLink {
    pub from_slot: usize,
    pub to_slot: usize,
    pub familiarity: f32,
}

/// A generation failure, reported as the side stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchReport {
    pub title: &'static str,
    pub message: &'static str,
}

/// The bodies, teammate links and failure reports of a fresh match.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RaisedMatch {
    pub fighters: Vec<RaisedFighter>,
    pub links: Vec<TeammateLink>,
    pub reports: Vec<MatchReport>,
}

/// The inputs of a fresh match, while the rematch counter is still zero.
pub struct FreshMatch<'a> {
    pub slots: &'a [i32],
    pub per_team: i32,
    /// How many teams to form, at most.
    pub teams: i32,
    pub spawn_at: [f32; 3],
    /// The side stood from the company list instead of generated.
    pub player_team: i32,
    /// How many company entries exist. A player-side member at this index
    /// or past it is left out.
    pub company_entries: usize,
    /// The company entry, counted inside its team window, that is the
    /// player's own character.
    pub player_entry: Option<usize>,
    /// The event's gear tier: a generic opponent's outfit level, and the
    /// level byte of a hire-list profile.
    pub tier: i32,
    /// Forced outfit id of a hire-list team, indexed by team id.
    pub outfits: &'a [i32],
    /// The event's weapon count. Positive draws a weapon from the row below.
    pub event_weapon: i32,
    /// The game clock, in milliseconds. Created bodies act at this plus
    /// [`MATCH_ACT_GRACE_MS`].
    pub now_ms: u32,
}

/// Raise a fresh tournament match from its bracket.
///
/// Sides and places come from [`bracket_layout`]. The player's side is
/// placed from the company list: the player's own entry keeps the body it
/// already has, and every other entry in range is a company body. Every
/// other side is generated. `create` returning false stops that side, the
/// failed body and the ones after it on that side are left out, and the
/// failure is reported; earlier bodies on the side stay, and later sides
/// are still raised. The created opponents of one side are linked both
/// ways at [`TEAMMATE_FAMILIARITY`]. Role flags follow how many sides were
/// formed, the player's side included: hostile below three, otherwise a
/// free-for-all.
#[must_use]
pub fn raise_fresh_match(
    spec: &FreshMatch<'_>,
    mut create: impl FnMut(&OpponentAttempt) -> bool,
) -> RaisedMatch {
    let laid = bracket_layout(spec.slots, spec.per_team, spec.teams, spec.spawn_at);
    let flags = if laid.len() < 3 {
        HOSTILE
    } else {
        FREE_FOR_ALL
    };
    let mut raised = RaisedMatch::default();
    for team in &laid {
        if team.team == spec.player_team {
            raise_company(team, spec, &mut raised);
        } else {
            raise_opponents(team, spec, flags, &mut create, &mut raised);
        }
    }
    raised
}

fn raise_company(team: &BracketTeam, spec: &FreshMatch<'_>, raised: &mut RaisedMatch) {
    for (order, &(slot, place)) in team.members.iter().enumerate() {
        if order >= spec.company_entries {
            break;
        }
        let window = slot % spec.per_team as usize;
        let body = if spec.player_entry == Some(window) {
            RaisedBody::Player
        } else {
            RaisedBody::Company {
                condition: COMPANY_CONDITION,
                role: MANAGER_ROLE,
                act_at_ms: spec.now_ms.saturating_add(MATCH_ACT_GRACE_MS),
            }
        };
        raised.fighters.push(RaisedFighter {
            slot,
            team: team.team,
            place,
            body,
        });
    }
}

fn raise_opponents(
    team: &BracketTeam,
    spec: &FreshMatch<'_>,
    flags: u32,
    create: &mut impl FnMut(&OpponentAttempt) -> bool,
    raised: &mut RaisedMatch,
) {
    let mut spawned = Vec::new();
    for &(slot, place) in &team.members {
        let plan = opponent_plan(team.team, spec);
        let attempt = OpponentAttempt {
            slot,
            team: team.team,
            place,
            plan,
        };
        if !create(&attempt) {
            raised.reports.push(MatchReport {
                title: MATCH_ERROR_TITLE,
                message: MATCH_ERROR,
            });
            break;
        }
        spawned.push(slot);
        raised.fighters.push(RaisedFighter {
            slot,
            team: team.team,
            place,
            body: RaisedBody::Opponent {
                plan,
                condition: OPPONENT_CONDITION,
                role: flags,
                act_at_ms: spec.now_ms.saturating_add(MATCH_ACT_GRACE_MS),
                behaviour: OPPONENT_BEHAVIOUR,
                action_cleared: true,
            },
        });
    }
    for &from in &spawned {
        for &to in &spawned {
            if from != to {
                raised.links.push(TeammateLink {
                    from_slot: from,
                    to_slot: to,
                    familiarity: TEAMMATE_FAMILIARITY,
                });
            }
        }
    }
}

fn opponent_plan(team: i32, spec: &FreshMatch<'_>) -> OpponentPlan {
    if team < 2 {
        OpponentPlan::Generic(generic_plan(5, spec.tier))
    } else {
        let company = usize::try_from(team)
            .ok()
            .and_then(|index| spec.outfits.get(index).copied())
            .unwrap_or(0);
        OpponentPlan::Hire(hire_plan(spec.tier as u8, company, spec.event_weapon))
    }
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
    fn a_bracket_groups_slots_into_teams() {
        // Two slots a team: the player (1) with two, a company (5) with one,
        // an empty run skipped, and a third team cut off at two teams.
        let slots = [1, 1, 0, 5, 0, 0, 7, 7];
        let teams = bracket_layout(&slots, 2, 2, [0.0; 3]);
        assert_eq!(teams.len(), 2);
        assert_eq!(teams[0].team, 1);
        assert_eq!(teams[1].team, 5);
        assert_eq!(
            teams[0].members.iter().map(|m| m.0).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            teams[1].members.iter().map(|m| m.0).collect::<Vec<_>>(),
            vec![1]
        );
        // Radius 2 × 2 × 50 + 100 = 300. Two members spread 2/3 from
        // −1/3; a lone member of the second team stands at a = 3, turn 0.
        let lone = teams[1].members[0].1;
        assert_eq!(lone.position, [0.0, 0.0, -300.0]);
        assert_eq!(lone.forward, [0.0, 0.0, 1.0]);
        let first = teams[0].members[0].1;
        let expected = bracket_place([0.0; 3], -f32::from_bits(0x3eaa_aaab), 300.0);
        assert_eq!(first, expected);
        // The radius stops at 350.
        let big = bracket_layout(&[2, 2, 2, 3, 3, 3], 3, 2, [0.0; 3]);
        let reach = big[0].members[1].1.position;
        assert!((reach[0].hypot(reach[2]) - 350.0).abs() < 1e-3);
        assert!(bracket_layout(&[0, 0], 2, 2, [0.0; 3]).is_empty());
        assert!(bracket_layout(&[1], 0, 2, [0.0; 3]).is_empty());
    }

    #[test]
    fn bracket_opponents_are_generic_or_a_company() {
        assert_eq!(
            bracket_opponent(1, 2, 3),
            Fighter::Opponent {
                level: 5,
                tier: 3,
                team: 1,
                flags: HOSTILE
            }
        );
        assert!(matches!(
            bracket_opponent(6, 3, 3),
            Fighter::Opponent {
                level: -1,
                team: 6,
                flags: FREE_FOR_ALL,
                ..
            }
        ));
    }

    #[test]
    fn the_next_round_brings_a_fresh_opponent_or_a_reserve() {
        assert_eq!(next_round(0, true), NextRound::FreshOpponent);
        assert_eq!(next_round(1, false), NextRound::FreshOpponent);
        assert_eq!(next_round(2, true), NextRound::Reserve(2));
    }

    fn fresh<'a>(slots: &'a [i32], per_team: i32, teams: i32) -> FreshMatch<'a> {
        FreshMatch {
            slots,
            per_team,
            teams,
            spawn_at: [0.0; 3],
            player_team: 1,
            company_entries: 8,
            player_entry: None,
            tier: 4,
            outfits: &[],
            event_weapon: 0,
            now_ms: 1_000,
        }
    }

    fn opponent(body: &RaisedBody) -> (&OpponentPlan, &ConditionWords, u32, u32, &BehaviourReset) {
        match body {
            RaisedBody::Opponent {
                plan,
                condition,
                role,
                act_at_ms,
                behaviour,
                action_cleared,
            } => {
                assert!(*action_cleared);
                (plan, condition, *role, *act_at_ms, behaviour)
            }
            other => panic!("expected an opponent, got {other:?}"),
        }
    }

    #[test]
    fn a_failed_body_stops_its_side_and_reports() {
        let slots = [4, 4, 4];
        let mut spec = fresh(&slots, 3, 1);
        spec.outfits = &[0, 0, 0, 0, 2];
        spec.event_weapon = 5;
        let mut asked = 0;
        let raised = raise_fresh_match(&spec, |attempt| {
            asked += 1;
            assert_eq!(attempt.slot, 0);
            assert_eq!(attempt.team, 4);
            match attempt.plan {
                OpponentPlan::Hire(plan) => {
                    assert_eq!(plan, hire_plan(4, 2, 5));
                    assert!(plan.draw_weapon);
                }
                OpponentPlan::Generic(_) => panic!("team 4 is a hire"),
            }
            false
        });
        assert_eq!(asked, 1);
        assert!(raised.fighters.is_empty());
        assert!(raised.links.is_empty());
        assert_eq!(
            raised.reports,
            vec![MatchReport {
                title: "Error",
                message: "Failed to generate match motile.",
            }]
        );
    }

    #[test]
    fn two_successes_are_related_both_ways_and_the_third_is_dropped() {
        let slots = [4, 4, 4];
        let spec = fresh(&slots, 3, 1);
        let mut n = 0;
        let raised = raise_fresh_match(&spec, |_| {
            n += 1;
            n < 3
        });
        assert_eq!(
            raised.fighters.iter().map(|f| f.slot).collect::<Vec<_>>(),
            vec![0, 1]
        );
        let (plan, condition, role, act_at, behaviour) = opponent(&raised.fighters[0].body);
        assert!(matches!(plan, OpponentPlan::Hire(_)));
        assert_eq!(*condition, OPPONENT_CONDITION);
        assert_eq!(role, HOSTILE);
        assert_eq!(act_at, 4_000);
        assert_eq!(*behaviour, OPPONENT_BEHAVIOUR);
        assert_eq!(
            raised.links,
            vec![
                TeammateLink {
                    from_slot: 0,
                    to_slot: 1,
                    familiarity: 100.0,
                },
                TeammateLink {
                    from_slot: 1,
                    to_slot: 0,
                    familiarity: 100.0,
                },
            ]
        );
        assert_eq!(raised.reports.len(), 1);
        assert_eq!(
            raised.fighters[0].place,
            bracket_layout(&slots, 3, 1, [0.0; 3])[0].members[0].1
        );
    }

    #[test]
    fn three_successes_link_every_ordered_pair() {
        let slots = [6, 6, 6];
        let raised = raise_fresh_match(&fresh(&slots, 3, 1), |_| true);
        assert_eq!(raised.links.len(), 6);
        assert!(raised.reports.is_empty());
        assert!(
            raised
                .links
                .iter()
                .all(|link| link.from_slot != link.to_slot)
        );
        assert!(raised.links.iter().all(|link| link.familiarity == 100.0));
    }

    #[test]
    fn three_sides_fight_free_and_two_stay_hostile() {
        let three = [2, 3, 4];
        let raised = raise_fresh_match(&fresh(&three, 1, 3), |_| true);
        assert!(raised.fighters.iter().all(|f| match f.body {
            RaisedBody::Opponent { role, .. } => role == FREE_FOR_ALL,
            _ => false,
        }));
        let two = [2, 3];
        let raised = raise_fresh_match(&fresh(&two, 1, 2), |_| true);
        assert!(raised.fighters.iter().all(|f| match f.body {
            RaisedBody::Opponent { role, .. } => role == HOSTILE,
            _ => false,
        }));
    }

    #[test]
    fn a_low_team_id_is_generic_level_five() {
        let slots = [0];
        let mut spec = fresh(&slots, 1, 1);
        spec.player_team = 99;
        spec.tier = 3;
        let raised = raise_fresh_match(&spec, |_| true);
        let (plan, _, _, _, _) = opponent(&raised.fighters[0].body);
        assert_eq!(plan, &OpponentPlan::Generic(generic_plan(5, 3)));
    }

    #[test]
    fn the_player_side_is_stood_and_not_generated() {
        let slots = [1, 0, 1, 2];
        let mut spec = fresh(&slots, 3, 2);
        spec.player_entry = Some(2);
        spec.company_entries = 2;
        let mut asked = Vec::new();
        let raised = raise_fresh_match(&spec, |attempt| {
            asked.push(attempt.slot);
            true
        });
        assert_eq!(asked, vec![3]);
        assert_eq!(raised.fighters.len(), 3);
        assert!(matches!(
            raised.fighters[0].body,
            RaisedBody::Company {
                role: MANAGER_ROLE,
                act_at_ms: 4_000,
                condition: COMPANY_CONDITION
            }
        ));
        assert!(matches!(raised.fighters[1].body, RaisedBody::Player));
        assert_eq!(raised.fighters[1].slot, 2);
        assert!(raised.links.is_empty());
        let (_, _, role, _, _) = opponent(&raised.fighters[2].body);
        assert_eq!(role, HOSTILE);
    }

    #[test]
    fn a_later_side_is_still_raised_after_a_failure() {
        let slots = [4, 4, 5];
        let raised = raise_fresh_match(&fresh(&slots, 2, 2), |attempt| attempt.team != 4);
        assert_eq!(
            raised.fighters.iter().map(|f| f.slot).collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(raised.reports.len(), 1);
        assert!(raised.links.is_empty());
    }

    #[test]
    fn an_unknown_outfit_forces_nothing() {
        let slots = [8];
        let raised = raise_fresh_match(&fresh(&slots, 1, 1), |_| true);
        let (plan, _, _, _, _) = opponent(&raised.fighters[0].body);
        assert_eq!(plan, &OpponentPlan::Hire(hire_plan(4, 0, 0)));
    }
}
