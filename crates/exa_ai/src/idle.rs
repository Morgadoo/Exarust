//! Idle roam, patrol and sentry.
//!
//! A roam waits, walks to a wander point, waits 0.5–2.5 s, walks to a
//! home-side point, then waits 1–5 s and repeats. A patrol waits 0.5–2.5 s
//! and walks to a wander point, then repeats. A sentry stands in the guard
//! pose. During a wait, a disturbance above 0.3 is walked to before the
//! alert look. An alert look holds the phase where it is.
//!
//! Evidence: the roam, patrol and sentry ticks. The waits are
//! `500 + draw(2000)` ms after a wander arrival and `1000 + draw(4000)` ms
//! after a home arrival. A deadline equal to the clock does not end the wait.

use bevy::math::Vec3;

use crate::hear::{self, Attention};

/// Which idle is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Roam,
    Patrol,
    Sentry,
}

/// Where the cycle is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Standing until the clock passes the deadline.
    Wait,
    /// Asking for a path to a wander point.
    PlanWander,
    /// Following that path.
    WalkWander,
    /// The pause between the wander point and the home-side point.
    WaitHome,
    /// Asking for a path to the home side.
    PlanHome,
    /// Following that path.
    WalkHome,
}

/// The pose the tick asks for. A preempted tick asks for none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    /// The resting stand.
    Stand,
    /// The sentry's guard.
    Guard,
}

/// Where the body should go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Going {
    Stay,
    Wander,
    Home,
    Investigate(Vec3),
    FaceTarget,
    LookAround,
}

/// One tick's pose and movement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Act {
    pub pose: Option<Pose>,
    pub going: Going,
}

/// The cycle and the clock it is waiting on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Routine {
    pub kind: Kind,
    pub phase: Phase,
    /// Wait until the clock is strictly past this, in milliseconds.
    pub until_ms: u32,
    pub roll: u32,
}

impl Routine {
    /// A roam standing until `until_ms`.
    pub fn roam(until_ms: u32) -> Self {
        Self {
            kind: Kind::Roam,
            phase: Phase::Wait,
            until_ms,
            roll: 0,
        }
    }

    /// A patrol standing until `until_ms`.
    pub fn patrol(until_ms: u32) -> Self {
        Self {
            kind: Kind::Patrol,
            phase: Phase::Wait,
            until_ms,
            roll: 0,
        }
    }

    /// A sentry. It does not walk.
    pub fn sentry() -> Self {
        Self {
            kind: Kind::Sentry,
            phase: Phase::Wait,
            until_ms: 0,
            roll: 0,
        }
    }
}

/// What the world reported this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moment {
    pub now_ms: u32,
    pub disturbance: f32,
    pub noise_at: Vec3,
    pub alert: f32,
    pub target_off: Option<f32>,
    pub looked_for_s: f32,
    pub look_gap_s: f32,
    /// The path request was accepted.
    pub path_accepted: bool,
    /// The current walk has reached its point.
    pub arrived: bool,
}

impl Default for Moment {
    fn default() -> Self {
        Self {
            now_ms: 0,
            disturbance: 0.0,
            noise_at: Vec3::ZERO,
            alert: 0.0,
            target_off: None,
            looked_for_s: 0.0,
            look_gap_s: hear::LOOK_SOON,
            path_accepted: false,
            arrived: false,
        }
    }
}

fn pause(roll: &mut u32, span: u32, base: u32, now: u32) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    let extra = ((u64::from(*roll) * u64::from(span)) >> 32) as u32;
    now.wrapping_add(base).wrapping_add(extra)
}

fn notice(routine: &Routine, moment: &Moment) -> Option<Going> {
    if routine.kind == Kind::Sentry {
        return None;
    }
    let waiting = match routine.kind {
        Kind::Patrol => routine.phase == Phase::Wait,
        Kind::Roam => matches!(routine.phase, Phase::Wait | Phase::WaitHome),
        Kind::Sentry => false,
    };
    if waiting
        && let Attention::Investigate(point) = hear::attention(
            moment.disturbance,
            moment.noise_at,
            moment.alert,
            moment.target_off,
            moment.looked_for_s,
            moment.look_gap_s,
        )
    {
        return Some(Going::Investigate(point));
    }
    match hear::attention(
        0.0,
        moment.noise_at,
        moment.alert,
        moment.target_off,
        moment.looked_for_s,
        moment.look_gap_s,
    ) {
        Attention::FaceTarget => Some(Going::FaceTarget),
        Attention::LookAround => Some(Going::LookAround),
        _ => None,
    }
}

fn finish_wander(routine: &mut Routine, now: u32) {
    routine.until_ms = pause(&mut routine.roll, 2000, 500, now);
    routine.phase = match routine.kind {
        Kind::Roam => Phase::WaitHome,
        _ => Phase::Wait,
    };
}

fn finish_home(routine: &mut Routine, now: u32) {
    routine.until_ms = pause(&mut routine.roll, 4000, 1000, now);
    routine.phase = Phase::Wait;
}

/// One idle tick. A sentry guards. A roam or a patrol stands, then walks.
pub fn tick(routine: &mut Routine, moment: &Moment) -> Act {
    if routine.kind == Kind::Sentry {
        return Act {
            pose: Some(Pose::Guard),
            going: Going::Stay,
        };
    }
    if let Some(going) = notice(routine, moment) {
        return Act { pose: None, going };
    }
    let mut going = Going::Stay;
    match routine.phase {
        Phase::Wait | Phase::WaitHome => {
            if routine.until_ms < moment.now_ms {
                routine.phase = if routine.phase == Phase::WaitHome {
                    Phase::PlanHome
                } else {
                    Phase::PlanWander
                };
            }
        }
        Phase::PlanWander => {
            going = Going::Wander;
            if moment.path_accepted {
                routine.phase = Phase::WalkWander;
            }
        }
        Phase::WalkWander => {
            going = Going::Wander;
            if moment.arrived {
                finish_wander(routine, moment.now_ms);
                going = Going::Stay;
            }
        }
        Phase::PlanHome if routine.kind == Kind::Roam => {
            going = Going::Home;
            if moment.path_accepted {
                routine.phase = Phase::WalkHome;
            }
        }
        Phase::WalkHome if routine.kind == Kind::Roam => {
            going = Going::Home;
            if moment.arrived {
                finish_home(routine, moment.now_ms);
                going = Going::Stay;
            }
        }
        Phase::PlanHome | Phase::WalkHome => {}
    }
    Act {
        pose: Some(Pose::Stand),
        going,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet(now_ms: u32) -> Moment {
        Moment {
            now_ms,
            ..Moment::default()
        }
    }

    #[test]
    fn a_roam_walks_out_and_back_and_a_sentry_guards() {
        let guard = tick(&mut Routine::sentry(), &quiet(0));
        assert_eq!(
            guard,
            Act {
                pose: Some(Pose::Guard),
                going: Going::Stay,
            }
        );

        let mut roam = Routine::roam(1000);
        let held = tick(&mut roam, &quiet(1000));
        assert_eq!(held.going, Going::Stay);
        assert_eq!(roam.phase, Phase::Wait);

        let left = tick(&mut roam, &quiet(1001));
        assert_eq!(left.pose, Some(Pose::Stand));
        assert_eq!(left.going, Going::Stay);
        assert_eq!(roam.phase, Phase::PlanWander);

        let mut asking = quiet(1001);
        asking.path_accepted = true;
        let seek = tick(&mut roam, &asking);
        assert_eq!(seek.going, Going::Wander);
        assert_eq!(roam.phase, Phase::WalkWander);

        let mut there = quiet(2000);
        there.arrived = true;
        let paused = tick(&mut roam, &there);
        assert_eq!(paused.going, Going::Stay);
        assert_eq!(roam.phase, Phase::WaitHome);
        assert_eq!(roam.until_ms, 2500);

        let toward_home = tick(&mut roam, &quiet(2501));
        assert_eq!(toward_home.going, Going::Stay);
        assert_eq!(roam.phase, Phase::PlanHome);
        let mut home = quiet(2501);
        home.path_accepted = true;
        assert_eq!(tick(&mut roam, &home).going, Going::Home);
        assert_eq!(roam.phase, Phase::WalkHome);
        let mut back = quiet(3000);
        back.arrived = true;
        assert_eq!(tick(&mut roam, &back).going, Going::Stay);
        assert_eq!(roam.phase, Phase::Wait);
        assert_eq!(roam.until_ms, 4125);

        let mut patrol = Routine::patrol(0);
        tick(&mut patrol, &quiet(1));
        let mut step = quiet(1);
        step.path_accepted = true;
        tick(&mut patrol, &step);
        let mut done = quiet(10);
        done.arrived = true;
        tick(&mut patrol, &done);
        assert_eq!(patrol.phase, Phase::Wait);
        assert_eq!(patrol.until_ms, 510);
        tick(&mut patrol, &quiet(511));
        assert_eq!(patrol.phase, Phase::PlanWander);

        let mut listening = Routine::roam(5_000);
        let mut noise = quiet(0);
        noise.disturbance = 0.31;
        noise.noise_at = Vec3::new(12.0, 0.0, 0.0);
        let heard = tick(&mut listening, &noise);
        assert_eq!(heard.pose, None);
        assert_eq!(heard.going, Going::Investigate(Vec3::new(12.0, 0.0, 0.0)));
        assert_eq!(listening.phase, Phase::Wait);
    }
}
