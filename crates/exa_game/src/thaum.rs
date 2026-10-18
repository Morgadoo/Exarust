//! One playable effect for each thaumaturgy domain.
//!
//! Displacement is the long door: the body is placed at the chosen point.
//! Energy is shock: yellow health drops by the arc's strength, and the same
//! body is not shocked again until 150 ms have passed. Force is blast: the
//! hit's knock is the cap of 30 doubled, along the horizontal aim. Mind is
//! calm: each concern drops by 30, aggression by 10 and fear by 50, floored
//! at zero. A cast needs focus of cost times its factor and spends that much.
//!
//! Factors: long door and shock 1, blast 1.2, calm 0.3. Costs: long door
//! 0.20, shock 0.10, blast 0.10, calm 0.14. The long-door factor is the
//! shared instant factor; vanish's 80× factor is a different power.
//!
//! Evidence: the power tables and the calm, blast and shock notes.

use bevy::math::Vec3;

/// The four domains that have a cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Displacement,
    Energy,
    Force,
    Mind,
}

pub const LONG_DOOR_COST: f32 = 0.20;
pub const SHOCK_COST: f32 = 0.10;
pub const BLAST_COST: f32 = 0.10;
pub const CALM_COST: f32 = 0.14;
pub const BLAST_FACTOR: f32 = 1.2;
pub const CALM_FACTOR: f32 = 0.3;
/// Milliseconds before the same body can be shocked again.
pub const SHOCK_GAP_MS: u32 = 150;
/// The blast knock cap, then doubled.
pub const BLAST_KNOCK: f32 = 30.0;

/// Focus the caster still has, and when shock last landed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caster {
    pub focus: f32,
    pub shock_at_ms: Option<u32>,
}

impl Caster {
    pub fn new(focus: f32) -> Self {
        Self {
            focus,
            shock_at_ms: None,
        }
    }
}

/// What the cast is aimed at.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub point: Vec3,
    pub aim: Vec3,
    pub yellow: f32,
    pub arc: f32,
    pub now_ms: u32,
    pub concerns: Vec<i32>,
    pub aggression: i32,
    pub fear: i32,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            point: Vec3::ZERO,
            aim: Vec3::Z,
            yellow: 0.25,
            arc: 0.0,
            now_ms: 0,
            concerns: Vec::new(),
            aggression: 0,
            fear: 0,
        }
    }
}

/// What changed.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Displacement {
        at: Vec3,
    },
    Energy {
        yellow: f32,
    },
    Force {
        impulse: Vec3,
    },
    Mind {
        concerns: Vec<i32>,
        aggression: i32,
        fear: i32,
    },
}

/// Why the cast did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastError {
    /// Focus is below cost times the factor.
    Focus,
    /// Blast has no horizontal aim.
    Aim,
}

fn spend(caster: &mut Caster, cost: f32, factor: f32) -> Result<(), CastError> {
    let need = cost * factor;
    if !caster.focus.is_finite() || caster.focus < need {
        return Err(CastError::Focus);
    }
    caster.focus -= need;
    Ok(())
}

fn floor_sub(value: i32, by: i32) -> i32 {
    value.saturating_sub(by).max(0)
}

/// Cast one domain's effect. Nothing changes when this returns an error.
pub fn cast(domain: Domain, caster: &mut Caster, request: &Request) -> Result<Effect, CastError> {
    match domain {
        Domain::Displacement => {
            spend(caster, LONG_DOOR_COST, 1.0)?;
            Ok(Effect::Displacement { at: request.point })
        }
        Domain::Energy => {
            spend(caster, SHOCK_COST, 1.0)?;
            let ready = caster
                .shock_at_ms
                .is_none_or(|then| request.now_ms.saturating_sub(then) >= SHOCK_GAP_MS);
            let loss = if ready && request.arc.is_finite() && request.arc > 0.0 {
                caster.shock_at_ms = Some(request.now_ms);
                request.arc
            } else {
                0.0
            };
            let yellow = if request.yellow.is_finite() {
                (request.yellow - loss).max(0.0)
            } else {
                0.0
            };
            Ok(Effect::Energy { yellow })
        }
        Domain::Force => {
            let flat = Vec3::new(request.aim.x, 0.0, request.aim.z);
            if flat.length_squared() <= f32::EPSILON {
                return Err(CastError::Aim);
            }
            spend(caster, BLAST_COST, BLAST_FACTOR)?;
            Ok(Effect::Force {
                impulse: flat.normalize() * (BLAST_KNOCK * 2.0),
            })
        }
        Domain::Mind => {
            spend(caster, CALM_COST, CALM_FACTOR)?;
            Ok(Effect::Mind {
                concerns: request
                    .concerns
                    .iter()
                    .copied()
                    .map(|concern| floor_sub(concern, 30))
                    .collect(),
                aggression: floor_sub(request.aggression, 10),
                fear: floor_sub(request.fear, 50),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_domain_changes_a_different_thing() {
        let mut door = Caster::new(LONG_DOOR_COST - f32::EPSILON);
        let point = Vec3::new(1.0, 2.0, 3.0);
        let mut request = Request {
            point,
            ..Request::default()
        };
        assert_eq!(
            cast(Domain::Displacement, &mut door, &request),
            Err(CastError::Focus)
        );
        assert!((door.focus - (LONG_DOOR_COST - f32::EPSILON)).abs() < 1e-6);
        door.focus = 1.0;
        match cast(Domain::Displacement, &mut door, &request).unwrap() {
            Effect::Displacement { at } => assert_eq!(at, point),
            other => panic!("{other:?}"),
        }
        assert!((door.focus - 0.80).abs() < 1e-5);

        let mut shock = Caster::new(1.0);
        request.yellow = 0.25;
        request.arc = 0.04;
        request.now_ms = 1_000;
        match cast(Domain::Energy, &mut shock, &request).unwrap() {
            Effect::Energy { yellow } => assert!((yellow - 0.21).abs() < 1e-5),
            other => panic!("{other:?}"),
        }
        request.now_ms = 1_100;
        request.yellow = 0.21;
        match cast(Domain::Energy, &mut shock, &request).unwrap() {
            Effect::Energy { yellow } => assert!((yellow - 0.21).abs() < 1e-5),
            other => panic!("{other:?}"),
        }
        request.now_ms = 1_150;
        match cast(Domain::Energy, &mut shock, &request).unwrap() {
            Effect::Energy { yellow } => assert!((yellow - 0.17).abs() < 1e-5),
            other => panic!("{other:?}"),
        }

        let mut blast = Caster::new(BLAST_COST * BLAST_FACTOR - 0.001);
        request.aim = Vec3::ZERO;
        assert_eq!(
            cast(Domain::Force, &mut blast, &request),
            Err(CastError::Aim)
        );
        request.aim = Vec3::new(3.0, 9.0, 0.0);
        assert_eq!(
            cast(Domain::Force, &mut blast, &request),
            Err(CastError::Focus)
        );
        blast.focus = 1.0;
        match cast(Domain::Force, &mut blast, &request).unwrap() {
            Effect::Force { impulse } => {
                assert!((impulse - Vec3::new(60.0, 0.0, 0.0)).length() < 1e-4);
            }
            other => panic!("{other:?}"),
        }

        let mut calm = Caster::new(CALM_COST * CALM_FACTOR);
        request.concerns = vec![40, 10];
        request.aggression = 15;
        request.fear = 40;
        match cast(Domain::Mind, &mut calm, &request).unwrap() {
            Effect::Mind {
                concerns,
                aggression,
                fear,
            } => {
                assert_eq!(concerns, vec![10, 0]);
                assert_eq!(aggression, 5);
                assert_eq!(fear, 0);
            }
            other => panic!("{other:?}"),
        }
        assert!(calm.focus.abs() < 1e-5);
        assert_eq!(
            cast(Domain::Mind, &mut calm, &request),
            Err(CastError::Focus)
        );
    }
}
