//! The blow an NPC picks once a fight has started.
//!
//! Unarmed, inside eight tenths of reach, two draws in five are an overhead
//! and the rest may step to the other side. Armed and stepping forward
//! with a body scale under 1.3, an old fighter or open hands take the
//! overhead; otherwise the thrust preference is drawn against eleven. A
//! facing score above 4 takes the other side. Beyond eight tenths of reach
//! the same score can thrust, and a quadruped thrusts on one draw in four.
//! Energy under eight tenths of the maximum refuses the swing. The swing's
//! body turn is (draw + 0.5) times the weapon's turn amount.
//!
//! The facing score is `(draw(7) + (5·skill + 5)·Δ) − 3` plus a fifth of
//! the parry bias, the bias added in double precision; the reach, scale,
//! crouch and energy tests compare in double precision, as the original's
//! do.
//!
//! Evidence: the combat state's attack choice ([`crate::combat`] runs it).
//! The clip index within a side was not read, so a side plays that side's
//! first clip.

use exa_combat::attack::AttackMotion;

use crate::combat::wrap;

/// What the fighter and the target are when the blow is chosen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Situation {
    /// Already inside an attack. A down or deeply crouched target asks for
    /// an overhead when that target is not running.
    pub attacking: bool,
    pub target_down: bool,
    /// The target is braced, which with [`Self::target_down`] asks for an overhead.
    pub target_supported: bool,
    pub target_crouch: f32,
    /// The target runs.
    pub target_stepping: bool,
    /// Weapon class is the fists.
    pub unarmed: bool,
    /// The hands class has no weapon bits.
    pub open_hands: bool,
    pub quadruped: bool,
    /// Chosen footwork is the step straight at the target.
    pub forward_step: bool,
    pub distance_cm: f32,
    pub reach_cm: f32,
    /// Body scale. A forward step under 1.3 uses the close choice.
    pub speed: f32,
    pub yaw: f32,
    /// Yaw of the footwork candidate the score is measured against.
    pub aim_yaw: f32,
    pub skill: f32,
    /// The fighter's parry bias.
    pub phase: i32,
    /// Thrust preference, 1 to 9.
    pub thrust_pref: i32,
    pub age: f32,
    pub energy: f32,
    pub energy_max: f32,
    /// How far the body turns through the swing.
    pub swing_turn: f32,
}

impl Default for Situation {
    fn default() -> Self {
        Self {
            attacking: false,
            target_down: false,
            target_supported: false,
            target_crouch: 0.0,
            target_stepping: false,
            unarmed: false,
            open_hands: false,
            quadruped: false,
            forward_step: false,
            distance_cm: 0.0,
            reach_cm: 50.0,
            speed: 0.0,
            yaw: 0.0,
            aim_yaw: 0.0,
            skill: 0.0,
            phase: 0,
            thrust_pref: 1,
            age: 0.0,
            energy: 1.0,
            energy_max: 1.0,
            swing_turn: 1.0,
        }
    }
}

/// The blow, and how far the body turns while it plays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Choice {
    pub motion: AttackMotion,
    pub turn: f32,
}

/// The next draw of the 32-bit generator, then the high half of that times `n`.
fn draw(roll: &mut u32, n: u32) -> i32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*roll) * u64::from(n)) >> 32) as i32
}

fn unit(roll: &mut u32) -> f32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    (*roll as f64 * 2f64.powi(-32)) as f32
}

/// The target is beyond eight tenths of reach (in double precision).
fn beyond_close(situation: &Situation) -> bool {
    f64::from(situation.reach_cm) * 0.8 < f64::from(situation.distance_cm)
}

/// The body scale is under 1.3 (in double precision).
fn small(situation: &Situation) -> bool {
    f64::from(situation.speed) < 1.3
}

fn close_blow(situation: &Situation, roll: &mut u32) -> AttackMotion {
    if situation.age >= 0.5 || situation.open_hands {
        AttackMotion::Overhead
    } else if draw(roll, 11) < situation.thrust_pref {
        AttackMotion::Stab
    } else {
        AttackMotion::Overhead
    }
}

fn scored_blow(situation: &Situation, roll: &mut u32) -> AttackMotion {
    let delta = wrap(situation.yaw + -situation.aim_yaw);
    let turned = (situation.skill * 5.0 + 5.0) * delta;
    let raw = draw(roll, 7) as f32 + turned - 3.0;
    let score = (f64::from(situation.phase) * 0.2 + f64::from(raw)) as f32;
    if score > 4.0 {
        return AttackMotion::Other(0);
    }
    if score <= -4.0 {
        return AttackMotion::Main(0);
    }
    let mut motion = AttackMotion::Main(0);
    if beyond_close(situation) {
        if situation.quadruped {
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            if *roll >> 30 == 0 {
                motion = AttackMotion::Stab;
            }
        } else {
            let room = (situation.thrust_pref as u32 >> 2) as i32 + 2;
            if draw(roll, 6) < room && small(situation) {
                motion = close_blow(situation, roll);
            }
        }
    }
    let taken = matches!(motion, AttackMotion::Overhead | AttackMotion::Stab);
    if !taken && (draw(roll, 9) as f32) < 4.0 + score {
        motion = AttackMotion::Other(0);
    }
    motion
}

fn pick(situation: &Situation, roll: &mut u32) -> AttackMotion {
    if situation.unarmed {
        let near = f64::from(situation.distance_cm) < f64::from(situation.reach_cm) * 0.8;
        if !near || draw(roll, 5) > 1 {
            if draw(roll, 7) < 3 {
                AttackMotion::Other(0)
            } else {
                AttackMotion::Main(0)
            }
        } else {
            AttackMotion::Overhead
        }
    } else if (situation.target_down && situation.target_supported)
        || f64::from(situation.target_crouch) > 0.9
    {
        AttackMotion::Overhead
    } else if !situation.forward_step || !small(situation) {
        scored_blow(situation, roll)
    } else {
        close_blow(situation, roll)
    }
}

/// The blow to start, or nothing when the fighter is already swinging with
/// no overhead to add, or when its energy is under eight tenths of the maximum.
pub fn choose_attack(situation: &Situation, roll: &mut u32) -> Option<Choice> {
    choose_ready_attack(situation, true, roll)
}

/// [`choose_attack`] with the attack executor's answer: when it refuses
/// (`ready` false) nothing starts and the swing turn is not drawn.
pub fn choose_ready_attack(situation: &Situation, ready: bool, roll: &mut u32) -> Option<Choice> {
    if situation.attacking {
        if (situation.target_down || f64::from(situation.target_crouch) > 0.9)
            && !situation.target_stepping
        {
            return Some(Choice {
                motion: AttackMotion::Overhead,
                turn: 0.0,
            });
        }
        return None;
    }
    let motion = pick(situation, roll);
    if f64::from(situation.energy_max) * 0.8 > f64::from(situation.energy) || !ready {
        return None;
    }
    Some(Choice {
        motion,
        turn: (unit(roll) + 0.5) * situation.swing_turn,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reach_and_the_score_pick_the_blow() {
        let mut close = Situation {
            unarmed: true,
            distance_cm: 10.0,
            reach_cm: 50.0,
            ..Situation::default()
        };
        let mut roll = 0;
        let choice = choose_attack(&close, &mut roll).unwrap();
        assert_eq!(choice.motion, AttackMotion::Overhead);

        close.distance_cm = 100.0;
        roll = 0;
        let choice = choose_attack(&close, &mut roll).unwrap();
        assert_eq!(choice.motion, AttackMotion::Other(0));

        close.energy = 0.79;
        roll = 0;
        assert!(choose_attack(&close, &mut roll).is_none());

        let mut armed = Situation {
            forward_step: true,
            thrust_pref: 9,
            ..Situation::default()
        };
        roll = 0;
        assert_eq!(
            choose_attack(&armed, &mut roll).unwrap().motion,
            AttackMotion::Stab
        );
        armed.age = 0.5;
        roll = 0;
        assert_eq!(
            choose_attack(&armed, &mut roll).unwrap().motion,
            AttackMotion::Overhead
        );

        let wide = Situation {
            phase: 100,
            ..Situation::default()
        };
        roll = 0;
        assert_eq!(
            choose_attack(&wide, &mut roll).unwrap().motion,
            AttackMotion::Other(0)
        );

        let beast = Situation {
            quadruped: true,
            distance_cm: 100.0,
            reach_cm: 50.0,
            ..Situation::default()
        };
        roll = 0;
        assert_eq!(
            choose_attack(&beast, &mut roll).unwrap().motion,
            AttackMotion::Stab
        );

        let down = Situation {
            attacking: true,
            target_down: true,
            ..Situation::default()
        };
        roll = 0;
        assert_eq!(
            choose_attack(&down, &mut roll).unwrap().motion,
            AttackMotion::Overhead
        );
        assert_eq!(roll, 0);
    }
}
