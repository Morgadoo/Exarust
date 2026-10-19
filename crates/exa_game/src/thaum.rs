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
//! The power table records each id and its focus cost. A sustained cost is
//! that cost times the tick in seconds, halved again with the sustain
//! technique. Repeating the same instant power with the repeat technique
//! halves the table cost. Blast and bolt spend 1.2 times the cost, barrier
//! twice, a mind power 0.3 times, and vanish 80 times.
//!
//! A blast charge rises by five sixths of the tick in seconds. Passing 1
//! releases it. Fizzling falls by twice the rate. A released chunk grows by
//! 0.0006 of its speed term, and two chunks closer than twice the first
//! radius move apart by half the overlap.
//!
//! A barrier ripple loses 0.05 of strength each step. The shown value moves
//! toward that strength by at most 0.04, and both below 0.07 removes it.
//! While it remains, its place eases 0.05 of the way toward the hit. The dome
//! extent is 30 plus 1.25 times the ripple count.
//!
//! Evidence: the power tables, the calm, blast and shock notes, the blast
//! charge step, and the dome ripple step.

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
/// The id bit of a power whose cost is per second.
pub const SUSTAINED: u32 = 0x0001_0000;
/// The id bit of a power that stays in a slot until cast again.
pub const PERSISTENT: u32 = 0x0002_0000;

/// One row of the power table: its id and its focus cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Power {
    pub id: u32,
    pub cost: f32,
}

/// The recorded power rows. A sustained id costs per second. A persistent
/// id occupies a slot.
pub fn catalog() -> &'static [Power] {
    &[
        Power {
            id: 0x101,
            cost: 0.13,
        },
        Power {
            id: 0x20101,
            cost: 0.27,
        },
        Power {
            id: 0x102,
            cost: 0.12,
        },
        Power {
            id: 0x20102,
            cost: 0.24,
        },
        Power {
            id: 0x10106,
            cost: 0.025,
        },
        Power {
            id: 0x103,
            cost: 0.14,
        },
        Power {
            id: 0x104,
            cost: 0.16,
        },
        Power {
            id: 0x105,
            cost: 0.17,
        },
        Power {
            id: 0x107,
            cost: 0.18,
        },
        Power {
            id: 0x108,
            cost: 0.18,
        },
        Power {
            id: 0x109,
            cost: 0.19,
        },
        Power {
            id: 0x2010A,
            cost: 0.29,
        },
        Power {
            id: 0x10B,
            cost: 0.19,
        },
        Power {
            id: 0x2010C,
            cost: 0.35,
        },
        Power {
            id: 0x2010D,
            cost: 0.25,
        },
        Power {
            id: 0x10F,
            cost: 0.30,
        },
        Power {
            id: 0x201,
            cost: 0.10,
        },
        Power {
            id: 0x202,
            cost: 0.10,
        },
        Power {
            id: 0x10203,
            cost: 0.10,
        },
        Power {
            id: 0x204,
            cost: 0.20,
        },
        Power {
            id: 0x10205,
            cost: 0.07,
        },
        Power {
            id: 0x20206,
            cost: 0.25,
        },
        Power {
            id: 0x20207,
            cost: 0.25,
        },
        Power {
            id: 0x20208,
            cost: 0.35,
        },
        Power {
            id: 0x10209,
            cost: 0.02,
        },
        Power {
            id: 0x20A,
            cost: 0.10,
        },
        Power {
            id: 0x2020B,
            cost: 0.30,
        },
        Power {
            id: 0x10601,
            cost: 0.10,
        },
        Power {
            id: 0x602,
            cost: 0.20,
        },
        Power {
            id: 0x20603,
            cost: 0.35,
        },
        Power {
            id: 0x604,
            cost: 0.35,
        },
        Power {
            id: 0x401,
            cost: 0.10,
        },
        Power {
            id: 0x2040B,
            cost: 0.30,
        },
    ]
}

/// Whether this id is paid per second.
pub fn is_sustained(id: u32) -> bool {
    id & SUSTAINED != 0
}

/// Whether this id occupies a power slot.
pub fn is_persistent(id: u32) -> bool {
    id & PERSISTENT != 0
}

/// What the cost is multiplied by before it is spent. Blast and bolt use
/// 1.2, barrier 2, a mind power 0.3, and vanish 80. The long door stays at 1.
pub fn spend_factor(id: u32) -> f32 {
    let kind = id & 0xFFFF;
    if kind == 0x0201 || kind == 0x0202 {
        return 1.2;
    }
    if kind == 0x0205 {
        return 2.0;
    }
    if kind == 0x0601 {
        return 80.0;
    }
    if id & 0xFF00 == 0x0100 {
        return 0.3;
    }
    1.0
}

/// The cost a cast actually asks for. A sustained power uses the tick length
/// in milliseconds, and the sustain technique halves that. Repeating the
/// same instant power with the repeat technique halves the table cost.
pub fn adjusted_cost(
    cost: f32,
    id: u32,
    tick_ms: f32,
    repeat_same: bool,
    repeat_skill: bool,
    sustain_skill: bool,
) -> f32 {
    if is_sustained(id) {
        let per_tick = cost * tick_ms / 1000.0;
        if sustain_skill {
            per_tick * 0.5
        } else {
            per_tick
        }
    } else if repeat_same && repeat_skill {
        cost * 0.5
    } else {
        cost
    }
}

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

/// The charge after one tick: the amount, the state, and whether it released.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Charge {
    pub amount: f32,
    pub state: i32,
    pub released: bool,
}

/// Five sixths of the tick in seconds.
pub fn charge_rate(tick_ms: u32) -> f32 {
    let step = (f64::from(tick_ms) * 0.001) as f32;
    (f64::from(step) * (5.0 / 6.0)) as f32
}

/// Advance a blast charge. State 0 charges, a negative state fizzles, and a
/// state of 1 or more has already released.
pub fn step_charge(amount: f32, rate: f32, state: i32) -> Charge {
    if state < 1 {
        if state < 0 {
            let mut amount = amount - rate * 2.0;
            let mut state = state;
            if amount < 0.0 {
                amount = 0.0;
                state = -2;
            }
            Charge {
                amount,
                state,
                released: false,
            }
        } else {
            let mut amount = amount + rate;
            let mut state = state;
            let mut released = false;
            if amount > 1.0 {
                amount = 0.0;
                released = true;
                if state == 0 {
                    state = 1;
                }
            }
            Charge {
                amount,
                state,
                released,
            }
        }
    } else {
        Charge {
            amount: amount + rate * 3.0,
            state,
            released: false,
        }
    }
}

/// The radius after one released tick. The speed term is 0.7 of the speed plus 3.
pub fn grow_radius(radius: f32, speed: f32, tick: f32) -> f32 {
    let term = speed * 0.7 + 3.0;
    (f64::from(tick) * f64::from(term) * 0.0006 + f64::from(radius)) as f32
}

/// Push two chunks apart when they are closer than twice the first radius.
/// A zero distance stays put.
pub fn separate_chunks(a: [f32; 3], b: [f32; 3], radius: f32) -> ([f32; 3], [f32; 3]) {
    let delta = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dist_sq = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    let span = radius + radius;
    if dist_sq == 0.0 || dist_sq >= span * span {
        return (a, b);
    }
    let dist = dist_sq.sqrt();
    let push = ((span - dist) / dist) * 0.5;
    let scaled = [delta[0] * push, delta[1] * push, delta[2] * push];
    (
        [a[0] + scaled[0], a[1] + scaled[1], a[2] + scaled[2]],
        [b[0] - scaled[0], b[1] - scaled[1], b[2] - scaled[2]],
    )
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

/// One ripple still on the dome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeRipple {
    pub shown: f32,
    pub strength: f32,
    pub at: [f32; 3],
}

/// 30 plus 1.25 for each ripple.
pub fn dome_extent(ripples: u32) -> f32 {
    ripples as f32 * 1.25 + 30.0
}

/// One step of a barrier ripple. `None` means it has faded off the dome.
pub fn step_ripple(
    shown: f32,
    strength: f32,
    at: [f32; 3],
    toward: [f32; 3],
) -> Option<DomeRipple> {
    let strength = strength - 0.05;
    let gap = (strength - shown).clamp(-0.04, 0.04);
    let shown = shown + gap;
    if shown < 0.07 && strength < 0.07 {
        return None;
    }
    Some(DomeRipple {
        shown,
        strength,
        at: [
            at[0] + (toward[0] - at[0]) * 0.05,
            at[1] + (toward[1] - at[1]) * 0.05,
            at[2] + (toward[2] - at[2]) * 0.05,
        ],
    })
}

/// Five thirds of the tick in seconds.
pub fn wave_rate(tick_ms: u32) -> f32 {
    let step = (f64::from(tick_ms) * 0.001) as f32;
    (f64::from(step) * (5.0 / 3.0)) as f32
}

/// The radius after one charging or fizzling tick.
pub fn charge_wave_radius(radius: f32) -> f32 {
    (f64::from(radius) + 0.0015) as f32
}

/// A released wave front after one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveFront {
    pub reach: f32,
    pub radius: f32,
    pub trail: f32,
}

/// One shell ripple still on the aura.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellRipple {
    pub shown: f32,
    pub strength: f32,
}

/// One step of a shell ripple. `None` means it has faded off the aura.
pub fn step_shell(shown: f32, strength: f32, scale: f32) -> Option<ShellRipple> {
    let strength = (f64::from(strength) - f64::from(scale) * 0.04) as f32;
    let hi = (f64::from(scale) * 0.06) as f32;
    let lo = (f64::from(scale) * -0.04) as f32;
    let shown = shown + (strength - shown).clamp(lo, hi);
    if shown < 0.05 && strength < 0.05 {
        None
    } else {
        Some(ShellRipple { shown, strength })
    }
}

/// A torrent's level and its two channels after one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorrentMix {
    pub level: f32,
    pub left: f32,
    pub right: f32,
    pub playing: bool,
}

/// Step a torrent. Below phase 1 the level follows the charge. Later it falls, and the channels ease or dim.
pub fn step_torrent(phase: i32, charge: f32, level: f32, left: f32, right: f32) -> TorrentMix {
    let (level, playing) = if phase < 1 {
        let product = (2.0 - charge) * charge;
        ((f64::from(product) * 0.4) as f32, true)
    } else {
        let level = (f64::from(level) - 0.01) as f32;
        (level, f64::from(level) >= 0.02)
    };
    let (left, right) = if phase == 1 {
        let gap = 0.5 - left;
        let left = (f64::from(gap) * 0.06 + f64::from(left)) as f32;
        let gap = (0.6 - f64::from(right)) as f32;
        let right = (f64::from(gap) * 0.06 + f64::from(right)) as f32;
        (left, right)
    } else if phase == 2 {
        (
            (f64::from(left) * 0.96) as f32,
            (f64::from(right) * 0.96) as f32,
        )
    } else {
        (left, right)
    };
    TorrentMix {
        level,
        left,
        right,
        playing,
    }
}

/// Reach grows by 0.6. Radius grows by 0.0006 of the speed. The trail is 0.07 of the speed plus 1.
pub fn release_wave(reach: f32, radius: f32, speed: f32) -> WaveFront {
    WaveFront {
        reach: (f64::from(reach) + 0.6) as f32,
        radius: (f64::from(speed) * 0.0006 + f64::from(radius)) as f32,
        trail: (f64::from(speed) * 0.07 + 1.0) as f32,
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

    #[test]
    fn a_blast_charges_then_the_chunks_separate() {
        assert_eq!(charge_rate(12), f32::from_bits(0x3c23_d70a));
        assert_eq!(charge_rate(1000), f32::from_bits(0x3f55_5555));

        let holding = step_charge(0.5, 0.5, 0);
        assert_eq!(holding.amount, 1.0);
        assert_eq!(holding.state, 0);
        assert!(!holding.released);

        let released = step_charge(0.6, 0.5, 0);
        assert_eq!(released.amount, 0.0);
        assert_eq!(released.state, 1);
        assert!(released.released);

        let fading = step_charge(0.5, 0.1, -1);
        assert_eq!(fading.amount, 0.3);
        assert_eq!(fading.state, -1);
        let spent = step_charge(0.3, 0.2, -1);
        assert_eq!(spent.amount, 0.0);
        assert_eq!(spent.state, -2);

        let after = step_charge(0.2, 0.1, 1);
        assert_eq!(after.amount, 0.5);
        assert!(!after.released);

        assert_eq!(grow_radius(0.0, 0.0, 1.0), f32::from_bits(0x3aeb_edfa));
        let (left, right) = separate_chunks([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0);
        assert_eq!(left, [-0.5, 0.0, 0.0]);
        assert_eq!(right, [1.5, 0.0, 0.0]);
        assert_eq!(
            separate_chunks([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 1.0),
            ([0.0, 0.0, 0.0], [2.0, 0.0, 0.0])
        );
        assert_eq!(
            separate_chunks([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], 1.0),
            ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn a_barrier_ripple_fades_toward_the_hit() {
        assert_eq!(dome_extent(0), 30.0);
        assert_eq!(dome_extent(4), 35.0);
        let ripple = step_ripple(1.0, 1.0, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]).expect("it holds");
        assert_eq!(ripple.shown, f32::from_bits(0x3f75_c28f));
        assert_eq!(ripple.strength, f32::from_bits(0x3f73_3333));
        assert_eq!(ripple.at, [0.5, 0.0, 0.0]);
        assert!(step_ripple(0.06, 0.06, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]).is_none());
        assert!(step_ripple(0.07, 0.07, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn a_wave_releases_sooner_and_the_front_steps() {
        assert_eq!(wave_rate(12), f32::from_bits(0x3ca3_d70a));
        assert_eq!(wave_rate(1000), f32::from_bits(0x3fd5_5555));

        let wave = step_charge(0.99, wave_rate(12), 0);
        assert!(wave.released);
        assert_eq!(wave.state, 1);
        assert_eq!(wave.amount, 0.0);
        let blast = step_charge(0.99, charge_rate(12), 0);
        assert!(!blast.released);
        assert_eq!(blast.amount, 1.0);
        assert_eq!(blast.state, 0);

        assert_eq!(charge_wave_radius(0.0), f32::from_bits(0x3ac4_9ba6));

        let still = release_wave(0.0, 0.0, 0.0);
        assert_eq!(still.reach, f32::from_bits(0x3f19_999a));
        assert_eq!(still.radius, 0.0);
        assert_eq!(still.trail, 1.0);
        let moving = release_wave(0.0, 0.0, 10.0);
        assert_eq!(moving.radius, f32::from_bits(0x3bc4_9ba6));
        assert_eq!(moving.trail, f32::from_bits(0x3fd9_999a));
    }

    #[test]
    fn a_shell_ripple_fades_by_its_scale() {
        let held = step_shell(1.0, 1.0, 1.0).expect("it holds");
        assert_eq!(held.shown, f32::from_bits(0x3f75_c28f));
        assert_eq!(held.strength, f32::from_bits(0x3f75_c28f));
        let wide = step_shell(1.0, 1.0, 2.0).expect("it holds");
        assert_eq!(wide.shown, f32::from_bits(0x3f6b_851f));
        assert_eq!(wide.strength, f32::from_bits(0x3f6b_851f));
        let edge = step_shell(0.09, 0.09, 1.0).expect("0.05 stays");
        assert_eq!(edge.shown, f32::from_bits(0x3d4c_ccce));
        assert_eq!(edge.strength, f32::from_bits(0x3d4c_ccce));
        assert!(step_shell(0.05, 0.05, 1.0).is_none());
    }

    #[test]
    fn a_torrent_follows_its_charge_then_dims() {
        let full = step_torrent(0, 1.0, 9.0, 3.0, 4.0);
        assert_eq!(full.level, f32::from_bits(0x3ecc_cccd));
        assert_eq!(full.left, 3.0);
        assert_eq!(full.right, 4.0);
        assert!(full.playing);
        let half = step_torrent(0, 0.5, 9.0, 0.0, 0.0);
        assert_eq!(half.level, f32::from_bits(0x3e99_999a));

        let eased = step_torrent(1, 0.4, 0.4, 0.0, 0.0);
        assert_eq!(eased.level, f32::from_bits(0x3ec7_ae15));
        assert_eq!(eased.left, f32::from_bits(0x3cf5_c28f));
        assert_eq!(eased.right, f32::from_bits(0x3d13_74bd));
        assert!(eased.playing);

        let dimmed = step_torrent(2, 0.0, 0.4, 1.0, 1.0);
        assert_eq!(dimmed.level, f32::from_bits(0x3ec7_ae15));
        assert_eq!(dimmed.left, f32::from_bits(0x3f75_c28f));
        assert_eq!(dimmed.right, f32::from_bits(0x3f75_c28f));
        assert!(dimmed.playing);

        let stopped = step_torrent(2, 0.0, 0.03, 1.0, 1.0);
        assert_eq!(stopped.level, f32::from_bits(0x3ca3_d70a));
        assert!(!stopped.playing);
    }

    #[test]
    fn the_table_prices_a_cast_and_a_sustained_tick() {
        let cost = |id: u32| catalog().iter().find(|power| power.id == id).unwrap().cost;
        assert!((cost(0x602) - LONG_DOOR_COST).abs() < 1e-6);
        assert!((cost(0x401) - SHOCK_COST).abs() < 1e-6);
        assert!((cost(0x201) - BLAST_COST).abs() < 1e-6);
        assert!((cost(0x103) - CALM_COST).abs() < 1e-6);
        assert_eq!(catalog().len(), 33);
        assert!(is_sustained(0x10205));
        assert!(is_persistent(0x2020B));
        assert!(!is_sustained(0x201));
        let barrier = cost(0x10205);
        assert!((adjusted_cost(barrier, 0x10205, 500.0, false, false, false) - 0.035).abs() < 1e-6);
        assert!((adjusted_cost(barrier, 0x10205, 500.0, false, false, true) - 0.0175).abs() < 1e-6);
        assert!((adjusted_cost(BLAST_COST, 0x201, 0.0, true, true, false) - 0.05).abs() < 1e-6);
        assert!((adjusted_cost(BLAST_COST, 0x201, 0.0, true, false, false) - 0.10).abs() < 1e-6);
        assert!((spend_factor(0x201) - BLAST_FACTOR).abs() < 1e-6);
        assert!((spend_factor(0x103) - CALM_FACTOR).abs() < 1e-6);
        assert!((spend_factor(0x10205) - 2.0).abs() < 1e-6);
        assert!((spend_factor(0x602) - 1.0).abs() < 1e-6);
        assert!((spend_factor(0x10601) - 80.0).abs() < 1e-6);
    }
}
