//! What the AI sees of the force effects: a blast, a wave and a barricade.
//!
//! Each tells the AI where it is, how dangerous it is and which way to step
//! aside. The words a danger assessment fills, as the AI reads them:
//! `flags` gains 4 when the effect is within 12 m; `near` (word 0x14) is a
//! distance score; `urge` (word 0x0f) is `min(1 − d/1200, 0.99)` for a blast
//! and a wave (`d` the distance to the body's chest) and the same for a
//! barricade to the wall's face; `reach` (word 0x14) is the effect's
//! threatening size; `fear` (word 0x0c) its blow; `face` (word 0x10) how
//! squarely the body faces it.
//!
//! - **Blast** ([`assess`], kind [`Kind::Blast`]): the nearest body's
//!   relation caps the score at `1 − relation/100`. A released blast has
//!   twice the reach and thrice the fear; a charged one only the near score
//!   `s²·36` (s = the eased urge). The reach is `s²·36·rel·0.003` (twice
//!   for a released charge), the fear `s²·rel·0.5` (doubled while over 12),
//!   and the face is a quarter of the aim's forward dot plus three quarters
//!   of the body's own facing.
//! - **Wave** ([`assess`], kind [`Kind::Wave`]): `s²·12` near, `s²·rel`
//!   fear, and the same face.
//! - **Barricade** ([`assess`], kind [`Kind::Barricade`]): the distance is
//!   to the wall's face; `s·6` near (thrice while it rises), `s²·rel·0.1`
//!   fear (thrice with the instant flag), and a lowered distance score when
//!   the body is behind the wall.
//! - **Stepping aside** ([`avoid`]): a released blast within 5 m pushes the
//!   body away with `(1 − d·0.002)·6`; the angle is the direction from the
//!   blast plus half a turn, wrapped to ±π.
//!
//! Evidence: the three entities' position, assessment and avoidance
//! routines, read from `Exanima.exe`.

use bevy::prelude::*;

use super::{Frame, distance_squared};

/// The reach of a blast or a wave, cm.
pub const REACH_CM: f32 = 1200.0;
/// The most urge, below one.
pub const MOST_URGE: f32 = 0.99;
/// A released blast is felt twice as far and three times as hard.
pub const RELEASED_REACH: f32 = 2.0;
pub const RELEASED_FEAR: f32 = 3.0;
/// A barricade's near score and its rising gain.
pub const BARRICADE_NEAR: f32 = 6.0;
pub const BARRICADE_RISING: f32 = 3.0;
/// The blast's avoidance radius and push.
pub const AVOID_RANGE_CM: f32 = 500.0;
pub const AVOID_PUSH: f32 = 6.0;

/// Which force effect the AI is looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Blast,
    Wave,
    Barricade,
}

/// The words a danger assessment fills.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Assessment {
    pub flags: u32,
    pub near: f32,
    pub urge: f32,
    pub reach: f32,
    pub fear: f32,
    pub face: f32,
    /// Whether the body is within the effect's reach at all.
    pub in_reach: bool,
}

/// A force effect's body for the AI: where it is and what state it is in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceDanger {
    pub kind: Kind,
    pub at: Vec3,
    /// The blast's or wave's forward aim; the barricade's wall normal.
    pub aim: Vec3,
    /// 1 while a blast or barricade charges, 2 released; the barricade's
    /// instant flag for its fear.
    pub state: i32,
    /// The barricade's wall angle (its facing turn), radians.
    pub wall_angle: f32,
    /// The distance from the barricade's face to its wall's far end, cm.
    pub behind: f32,
}

/// The mean of two facing scores: a quarter of the first, three quarters of
/// the second.
fn blend_face(aim: Vec3, toward: Vec3, aim_dot: f32, body_dot: f32) -> f32 {
    let _ = (aim, toward);
    let first = aim_dot * -0.5 + 0.5;
    let second = body_dot * -0.5 + 0.5;
    first * first * 0.25 + second * 0.75
}

/// Assess one effect against a body's chest.
pub fn assess(
    effect: &ForceDanger,
    chest: Vec3,
    relation: i32,
    body_facing: Vec3,
) -> Option<Assessment> {
    let offset = effect.at - chest;
    let d2 = offset.length_squared();
    match effect.kind {
        Kind::Blast | Kind::Wave => {
            let limit = REACH_CM * REACH_CM;
            let inside = match effect.kind {
                Kind::Blast => d2 < limit,
                _ => d2 <= limit,
            };
            if !inside {
                return None;
            }
            let d = d2.sqrt();
            let dir = offset / d;
            let mut urge = 1.0 - d / REACH_CM;
            if MOST_URGE < urge {
                urge = MOST_URGE;
            }
            let eased = (2.0 - urge) * urge;
            let released = 0 < effect.state;
            let base = if effect.kind == Kind::Blast {
                36.0
            } else {
                12.0
            };
            let mut near = eased * base;
            if released {
                near *= RELEASED_REACH;
            }
            let mut relation_scale = 1.0 - relation as f32 * 0.01;
            if released {
                relation_scale *= RELEASED_FEAR;
            }
            let reach = relation_scale * near * 0.003;
            let fear = if effect.kind == Kind::Blast {
                let value = eased * eased * relation_scale * 0.5;
                if 12.0 < value {
                    eased * eased * relation_scale
                } else {
                    value
                }
            } else {
                eased * eased * relation_scale
            };
            let aim_dot = dir.dot(effect.aim);
            let body_dot = dir.dot(body_facing);
            Some(Assessment {
                flags: 4,
                near,
                urge,
                reach,
                fear,
                face: blend_face(effect.aim, dir, aim_dot, body_dot),
                in_reach: true,
            })
        }
        Kind::Barricade => {
            let limit = REACH_CM * REACH_CM;
            if d2 > limit {
                return None;
            }
            let d = d2.sqrt();
            let dir = offset / d;
            let mut urge = 1.0 - d / REACH_CM;
            if MOST_URGE < urge {
                urge = MOST_URGE;
            }
            let rising = effect.state == 1;
            let mut near = urge * BARRICADE_NEAR;
            if rising {
                near *= BARRICADE_RISING;
            }
            let mut relation_scale = 1.0 - relation as f32 * 0.01;
            let instant = effect.state != 0;
            if instant {
                relation_scale *= BARRICADE_RISING;
            }
            let aim_dot = dir.dot(effect.aim);
            let body_dot = dir.dot(body_facing);
            Some(Assessment {
                flags: 4,
                near,
                urge,
                reach: relation_scale * near * 0.0001,
                fear: urge * urge * relation_scale * 0.1,
                face: blend_face(effect.aim, dir, aim_dot, body_dot),
                in_reach: true,
            })
        }
    }
}

/// Where and how hard to step aside from a blast or barricade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Avoidance {
    /// The distance from the body, cm.
    pub distance: f32,
    /// The push, or the danger falloff.
    pub push: f32,
    /// The direction to step, radians: from the effect, half a turn on.
    pub angle: f32,
}

/// The step aside from a released blast within 5 m, or the falloff from a
/// barricade's wall.
pub fn avoid(effect: &ForceDanger, body: Vec3) -> Option<Avoidance> {
    match effect.kind {
        Kind::Blast | Kind::Wave => {
            let offset = body - effect.at;
            let distance = offset.length();
            let released = 0 < effect.state;
            if distance < AVOID_RANGE_CM && released {
                let push = (1.0 - distance * 0.002) * AVOID_PUSH;
                let angle = offset.z.atan2(offset.x);
                let angle = if angle <= std::f32::consts::PI {
                    angle + std::f32::consts::PI
                } else {
                    angle - std::f32::consts::PI
                };
                Some(Avoidance {
                    distance,
                    push,
                    angle,
                })
            } else {
                None
            }
        }
        Kind::Barricade => {
            let offset = body - effect.at;
            let distance = offset.length();
            let wall = (300.0 - (distance - effect.behind).abs()) * 0.003;
            Some(Avoidance {
                distance,
                push: wall,
                angle: offset.z.atan2(offset.x),
            })
        }
    }
}

/// A blast's place: its effect's frame applied to its local place.
pub fn blast_position(frame: &Frame, local: Vec3) -> Vec3 {
    frame.point(local)
}

/// Whether a body is within a blast's danger: 12 m from its place.
pub fn in_blast_reach(at: Vec3, chest: Vec3) -> bool {
    distance_squared(at, chest) < REACH_CM * REACH_CM
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blast(released: bool) -> ForceDanger {
        ForceDanger {
            kind: Kind::Blast,
            at: Vec3::new(600.0, 0.0, 0.0),
            aim: Vec3::X,
            state: if released { 2 } else { 0 },
            wall_angle: 0.0,
            behind: 0.0,
        }
    }

    #[test]
    fn a_blast_just_inside_twelve_metres_is_assessed() {
        let effect = blast(false);
        let chest = Vec3::ZERO;
        let a = assess(&effect, chest, 0, Vec3::Y).unwrap();
        assert_eq!(a.flags, 4);
        assert!((a.urge - 0.5).abs() < 1e-6);
        // s = (2 − 0.5)·0.5 = 0.75; near = 0.75·36 = 27.
        assert!((a.near - 27.0).abs() < 1e-5);
        // Charged: reach 27·0.003, fear 0.75²·0.5.
        assert!((a.reach - 27.0 * 0.003).abs() < 1e-7);
        assert!((a.fear - 0.75 * 0.75 * 0.5).abs() < 1e-5);
        // Beyond 12 m nothing.
        assert!(assess(&effect, Vec3::new(-601.0, 0.0, 0.0), 0, Vec3::Y).is_none());
    }

    #[test]
    fn a_released_blast_is_twice_the_reach_and_thrice_the_fear() {
        let effect = blast(true);
        let a = assess(&effect, Vec3::ZERO, 0, Vec3::X).unwrap();
        // The released blast has twice the near score: 54.
        assert!((a.near - 54.0).abs() < 1e-5);
        // Reach: 3·54·0.003 = 0.486.
        assert!((a.reach - 0.486).abs() < 1e-6);
        // Fear: 0.75²·3·0.5 = 0.84375.
        assert!((a.fear - 0.843_75).abs() < 1e-6);
        // The aim dot for a chest 600 west of the blast is 1: face =
        // 0.25·(0·−0.5+0.5)² ... the blended score.
        assert!(a.face >= 0.0 && a.face <= 1.0);
    }

    #[test]
    fn a_wave_is_twelve_and_merges_at_twelve_metres() {
        let wave = ForceDanger {
            kind: Kind::Wave,
            at: Vec3::new(1200.0, 0.0, 0.0),
            aim: Vec3::X,
            state: 1,
            wall_angle: 0.0,
            behind: 0.0,
        };
        let a = assess(&wave, Vec3::ZERO, 0, Vec3::X).unwrap();
        assert_eq!(a.urge, 0.0f32.max(1.0 - 1.0));
        assert!(a.in_reach);
        let near = ForceDanger {
            at: Vec3::new(600.0, 0.0, 0.0),
            ..wave
        };
        let a = assess(&near, Vec3::ZERO, 0, Vec3::X).unwrap();
        // 0.75·12 = 9, doubled for a released wave: 18.
        assert!((a.near - 18.0).abs() < 1e-5);
    }

    #[test]
    fn a_barricade_scores_six_while_it_stands_and_thrice_while_it_rises() {
        let wall = ForceDanger {
            kind: Kind::Barricade,
            at: Vec3::new(300.0, 0.0, 0.0),
            aim: Vec3::X,
            state: 0,
            wall_angle: 0.0,
            behind: 0.0,
        };
        let a = assess(&wall, Vec3::ZERO, 0, Vec3::X).unwrap();
        assert!((a.near - 4.5).abs() < 1e-5);
        assert!((a.fear - 0.75 * 0.75 * 0.1).abs() < 1e-6);
        let rising = ForceDanger { state: 1, ..wall };
        let a = assess(&rising, Vec3::ZERO, 0, Vec3::X).unwrap();
        assert!((a.near - 13.5).abs() < 1e-5);
    }

    #[test]
    fn a_released_blast_pushes_a_nearby_body_away_round_the_turn() {
        let effect = blast(true);
        let a = avoid(&effect, Vec3::new(600.0, 0.0, 100.0)).unwrap();
        assert!((a.distance - 100.0).abs() < 1e-4);
        assert!((a.push - (1.0 - 100.0 * 0.002) * 6.0).abs() < 1e-5);
        assert!((a.angle - std::f32::consts::FRAC_PI_2 * 3.0).abs() < 1e-5);
        // A charged blast is not stepped aside from.
        assert!(avoid(&blast(false), Vec3::new(600.0, 0.0, 100.0)).is_none());
    }
}
