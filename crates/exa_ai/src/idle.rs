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
//! A wander point is a quarter of the way from a drawn exit toward the room
//! point. The draw retries once when that exit leads back. A follower stops
//! inside a facing-scaled 100 cm, and outside a 400 cm band it sets an
//! urgency and replans when the leader has moved more than 100 cm. Sleep
//! lies dormant: the dormant condition and flag, tone 0, a statue, and
//! gait 2. The tick after that changes nothing. Seeing a contact, and only
//! when the flags are exactly the dormant flag, knocks that sleeper out.

use bevy::math::{Vec2, Vec3};
use exa_combat::condition::{Condition, StateFlags};
use exa_physics::body::{Drive, PIN_GAIN, Ragdoll, WAKE_COUNT};

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

/// Stop distance of a wander goal, in centimetres.
pub const WANDER_STOP_CM: i32 = 20;
/// Radius of a wander goal, in centimetres.
pub const WANDER_RADIUS_CM: f32 = 150.0;
/// The mode a wander pick writes.
pub const WANDER_MODE: i32 = 2;
const WANDER_BLEND: f32 = 0.25;

/// One way out of the walker's room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exit {
    /// The exit on the horizontal plane. The second component is depth.
    pub point: Vec2,
    /// The room this exit leads to.
    pub node: i32,
}

/// The point a wander walk aims at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WanderPoint {
    pub goal: Vec3,
    pub stop_cm: i32,
    pub radius_cm: f32,
    pub mode: i32,
    pub node: i32,
    pub cursor: i32,
    /// The goal test ignores height.
    pub ignore_height: bool,
}

fn draw_exit(roll: &mut u32, count: usize) -> usize {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*roll).wrapping_mul(count as u64)) >> 32) as usize
}

/// Pick a wander point in `room_index`.
///
/// No room, or a room with no exits, picks nothing and does not step the
/// roll. Otherwise one exit is drawn. When that exit leads to `avoid`, one
/// more exit is drawn, and that second draw is kept even if it leads back
/// too. The goal is a quarter of the way from the exit toward `room`, at
/// height 0. The stop is 20 cm and the radius is 150 cm.
pub fn pick_wander(
    room_index: i32,
    exits: &[Exit],
    room: Vec2,
    avoid: i32,
    roll: &mut u32,
) -> Option<WanderPoint> {
    if room_index < 0 || exits.is_empty() {
        return None;
    }
    let mut index = draw_exit(roll, exits.len());
    if exits[index].node == avoid {
        index = draw_exit(roll, exits.len());
    }
    let exit = exits[index];
    let toward = exit.point + (room - exit.point) * WANDER_BLEND;
    Some(WanderPoint {
        goal: Vec3::new(toward.x, 0.0, toward.y),
        stop_cm: WANDER_STOP_CM,
        radius_cm: WANDER_RADIUS_CM,
        mode: WANDER_MODE,
        node: exit.node,
        cursor: index as i32,
        ignore_height: true,
    })
}

/// Personal stop is this many centimetres, scaled by how the leader faces the follower.
pub const FOLLOW_PERSONAL_CM: f32 = 100.0;
/// Follow band, in centimetres. Already following uses 0.7 of it. Busy uses 3.
pub const FOLLOW_BAND_CM: f32 = 400.0;
/// A chase plans to stop this many centimetres short of the leader.
pub const FOLLOW_PLAN_CM: i32 = 50;
/// Replan when the leader is farther than this, squared, from the planned spot.
const REPLAN_CM2: f32 = 10_000.0;
/// Already walking a follow path.
pub const FOLLOWING: i32 = 4;
/// Closing the last gap of a follow.
pub const CLOSING: i32 = 6;

/// What one follow step decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowAct {
    /// Stand. No chase.
    Stand,
    /// The idle activity runs.
    Activity,
    /// Inside the personal stop, and the plan was accepted.
    ClosePlanned,
    /// Inside the personal stop, then stood.
    CloseStood,
    /// Mode is closing, the leader is still outside the stop of the goal, and the follower is walking.
    Keep,
    /// Outside the band. Urgency and replan are on the follower.
    Pursue,
}

/// The follower's follow fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Follower {
    pub place: Vec3,
    pub mode: i32,
    pub walking: bool,
    pub busy: bool,
    pub panic: f32,
    pub has_activity: bool,
    pub goal: Vec3,
    pub planned: Vec3,
    pub urgency: i32,
    pub replan: bool,
    pub stop_cm: i32,
    pub hold_search: bool,
}

/// The leader, when there is one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leader {
    pub place: Vec3,
    /// The point a chase paths to.
    pub aim: Vec3,
    pub facing: Vec3,
    pub sprinting: bool,
}

fn personal_stop(follower: Vec3, leader: &Leader) -> f32 {
    let delta = follower - leader.place;
    let distance = delta.length();
    if distance == 0.0 {
        return FOLLOW_PERSONAL_CM;
    }
    let facing = leader.facing.dot(delta) / distance;
    (1.0 + facing * 0.5) * FOLLOW_PERSONAL_CM
}

/// One follow step.
///
/// No leader stands when there is no activity or panic is at least 1, and
/// otherwise runs the activity. Not walking, inside the personal stop,
/// closes. The band is 400 cm, 280 while already following, and 1200 while
/// busy. Outside it the urgency is 0 up to 800 cm, 1 up to 1600 cm, and 2
/// past 1600 cm or when the leader sprints. Already following, a walk
/// replans only when the leader is more than 100 cm from the planned spot.
/// A chase that replans aims at the leader and stops 50 cm short. The
/// obstacle nudge inside the band is not in this slice.
pub fn follow_leader(
    follower: &mut Follower,
    leader: Option<&Leader>,
    visible: bool,
    plan_accepted: bool,
) -> FollowAct {
    let Some(leader) = leader else {
        if !follower.has_activity || follower.panic >= 1.0 {
            follower.busy = false;
            return FollowAct::Stand;
        }
        return FollowAct::Activity;
    };
    let distance = (follower.place - leader.place).length();
    let stop = personal_stop(follower.place, leader);
    if (distance < stop && !follower.walking) || follower.mode == CLOSING {
        follower.urgency = 0;
        follower.busy = false;
        if follower.mode == CLOSING {
            let gap = (follower.goal - leader.place).length_squared();
            if stop * stop <= gap {
                if follower.walking {
                    return FollowAct::Keep;
                }
            } else {
                follower.replan = true;
            }
        } else {
            follower.replan = true;
        }
        if follower.replan && plan_accepted {
            return FollowAct::ClosePlanned;
        }
        return FollowAct::CloseStood;
    }
    let band = if follower.mode == FOLLOWING {
        FOLLOW_BAND_CM * 0.7
    } else if follower.busy {
        FOLLOW_BAND_CM * 3.0
    } else {
        FOLLOW_BAND_CM
    };
    if distance < band {
        if !follower.has_activity {
            follower.busy = false;
            return FollowAct::Stand;
        }
        return FollowAct::Activity;
    }
    follower.busy = false;
    if leader.sprinting || FOLLOW_BAND_CM * 4.0 < distance {
        follower.urgency = 2;
    } else if distance <= FOLLOW_BAND_CM * 2.0 {
        follower.urgency = 0;
    } else {
        follower.urgency = 1;
    }
    if follower.mode == FOLLOWING {
        if !follower.walking {
            if !visible {
                return FollowAct::Stand;
            }
            follower.replan = true;
        } else if REPLAN_CM2 < (follower.planned - leader.place).length_squared() {
            follower.replan = true;
        }
    } else {
        follower.replan = true;
    }
    if follower.replan {
        follower.goal = leader.aim;
        follower.stop_cm = FOLLOW_PLAN_CM;
        follower.hold_search = false;
        if plan_accepted {
            follower.mode = FOLLOWING;
            follower.planned = leader.place;
        }
    }
    FollowAct::Pursue
}

/// Gait a sleeper writes. The pelvis rest uses the taller offset at this value.
pub const SLEEP_GAIT: i32 = 2;
/// The sleep activity. Seeing a contact removes it.
pub const SLEEP_ROLE: u16 = 44;

/// Which tick is installed. Sleep's tick changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleTick {
    Active,
    Sleep,
}

/// The character fields sleep writes, beside the body's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sleeper {
    pub condition: Condition,
    pub flags: StateFlags,
    pub statue: bool,
    pub gait: i32,
    pub tick: IdleTick,
    /// Set when a contact wakes the sleeper.
    pub sighted: u8,
    /// The clock the knock-out wakes from.
    pub wake_at: i32,
}

/// Lie dormant.
///
/// The condition becomes dormant. The dormant flag is set and the other
/// flags stay. Tone becomes 0, so the tone target does too. The body is a
/// statue and its gait is 2. The tick installed afterwards changes nothing.
pub fn sleep(sleeper: &mut Sleeper, drive: &mut Drive) {
    sleeper.flags.0 |= StateFlags::DORMANT;
    sleeper.condition = Condition::DORMANT;
    drive.tone = 0.0;
    sleeper.statue = true;
    sleeper.gait = SLEEP_GAIT;
    sleeper.tick = IdleTick::Sleep;
}

/// The tick sleep installs. It does not write the sleeper or the tone.
pub fn sleep_tick(_sleeper: &mut Sleeper, _drive: &mut Drive) {}

/// Wake a dormant sleeper that has seen a contact.
///
/// The flags must be exactly the dormant flag, and a contact must have
/// been seen. Otherwise nothing changes. The sleep role is removed, the
/// body is knocked out at `now` (delay 0): it is down, the dormant flag
/// stays and the waking flag is set, the statue mark clears, gait is 0,
/// tone is 0, the pin gain is 0.08, and the sleep countdown is raised to
/// 256. Waking's later tone rise is not in this slice.
pub fn see_target(
    sleeper: &mut Sleeper,
    drive: &mut Drive,
    body: &mut Ragdoll,
    roles: &mut Vec<u16>,
    seen: bool,
    now: i32,
) -> bool {
    if sleeper.flags.0 != StateFlags::DORMANT || !seen {
        return false;
    }
    roles.retain(|role| *role != SLEEP_ROLE);
    sleeper.sighted = 1;
    sleeper.wake_at = now;
    sleeper.condition = Condition::DOWN;
    sleeper.flags.0 = (sleeper.flags.0 & !0xF0) | (StateFlags::DORMANT | StateFlags::WAKING);
    sleeper.statue = false;
    sleeper.gait = 0;
    drive.tone = 0.0;
    drive.pin_gain = PIN_GAIN;
    body.wake_to(WAKE_COUNT);
    true
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

    fn exit(x: f32, z: f32, node: i32) -> Exit {
        Exit {
            point: Vec2::new(x, z),
            node,
        }
    }

    #[test]
    fn a_wander_point_is_a_quarter_of_the_way_from_the_exit() {
        let mut roll = 1;
        assert!(pick_wander(-1, &[exit(0.0, 0.0, 5)], Vec2::ZERO, 9, &mut roll).is_none());
        assert_eq!(roll, 1);
        assert!(pick_wander(0, &[], Vec2::ZERO, 9, &mut roll).is_none());
        assert_eq!(roll, 1);

        let picked =
            pick_wander(0, &[exit(0.0, 0.0, 5)], Vec2::new(100.0, 0.0), 9, &mut roll).unwrap();
        assert_eq!(picked.goal, Vec3::new(25.0, 0.0, 0.0));
        assert_eq!(picked.stop_cm, 20);
        assert_eq!(picked.radius_cm, 150.0);
        assert_eq!(picked.mode, 2);
        assert_eq!(picked.node, 5);
        assert_eq!(picked.cursor, 0);
        assert!(picked.ignore_height);
        assert_eq!(roll, 134_775_814);

        let mut deep = 1;
        let picked = pick_wander(0, &[exit(0.0, 100.0, 5)], Vec2::ZERO, 9, &mut deep).unwrap();
        assert_eq!(picked.goal, Vec3::new(0.0, 0.0, 75.0));

        let mut retry = 1;
        let picked = pick_wander(
            0,
            &[exit(0.0, 0.0, 5), exit(200.0, 0.0, 7)],
            Vec2::ZERO,
            5,
            &mut retry,
        )
        .unwrap();
        assert_eq!(picked.cursor, 1);
        assert_eq!(picked.node, 7);
        assert_eq!(picked.goal, Vec3::new(150.0, 0.0, 0.0));
        assert_eq!(retry, 3_698_175_007);

        let mut same = 1;
        let picked =
            pick_wander(0, &[exit(0.0, 0.0, 5)], Vec2::new(100.0, 0.0), 5, &mut same).unwrap();
        assert_eq!(picked.cursor, 0);
        assert_eq!(picked.node, 5);
        assert_eq!(picked.goal, Vec3::new(25.0, 0.0, 0.0));
        assert_eq!(same, 3_698_175_007);
    }

    fn follower_at(z: f32) -> Follower {
        Follower {
            place: Vec3::new(0.0, 0.0, z),
            mode: 0,
            walking: false,
            busy: false,
            panic: 0.0,
            has_activity: true,
            goal: Vec3::ZERO,
            planned: Vec3::ZERO,
            urgency: 9,
            replan: false,
            stop_cm: 0,
            hold_search: true,
        }
    }

    fn leader_facing_ahead() -> Leader {
        Leader {
            place: Vec3::ZERO,
            aim: Vec3::new(1.0, 2.0, 3.0),
            facing: Vec3::Z,
            sprinting: false,
        }
    }

    #[test]
    fn a_follower_stops_by_facing_and_replans_past_one_hundred_centimetres() {
        let leader = leader_facing_ahead();
        let mut none = follower_at(0.0);
        none.has_activity = false;
        assert_eq!(
            follow_leader(&mut none, None, true, false),
            FollowAct::Stand
        );
        assert!(!none.busy);

        let mut panicked = follower_at(0.0);
        panicked.panic = 1.0;
        assert_eq!(
            follow_leader(&mut panicked, None, true, false),
            FollowAct::Stand
        );

        let mut calm = follower_at(0.0);
        calm.panic = 0.99;
        assert_eq!(
            follow_leader(&mut calm, None, true, false),
            FollowAct::Activity
        );

        let mut close = follower_at(149.0);
        assert_eq!(
            follow_leader(&mut close, Some(&leader), true, false),
            FollowAct::CloseStood
        );
        assert_eq!(close.urgency, 0);
        assert!(close.replan);
        assert!(!close.busy);

        let mut edge = follower_at(150.0);
        assert_eq!(
            follow_leader(&mut edge, Some(&leader), true, false),
            FollowAct::Activity
        );
        assert_eq!(edge.urgency, 9);

        let mut walk = follower_at(500.0);
        assert_eq!(
            follow_leader(&mut walk, Some(&leader), true, true),
            FollowAct::Pursue
        );
        assert_eq!(walk.urgency, 0);
        assert_eq!(walk.stop_cm, 50);
        assert_eq!(walk.goal, leader.aim);
        assert_eq!(walk.mode, FOLLOWING);
        assert_eq!(walk.planned, leader.place);
        assert!(!walk.hold_search);

        let mut jog = follower_at(801.0);
        assert_eq!(
            follow_leader(&mut jog, Some(&leader), true, false),
            FollowAct::Pursue
        );
        assert_eq!(jog.urgency, 1);
        assert_eq!(jog.mode, 0);

        let mut run = follower_at(1_601.0);
        assert_eq!(
            follow_leader(&mut run, Some(&leader), true, false),
            FollowAct::Pursue
        );
        assert_eq!(run.urgency, 2);

        let mut sprint = follower_at(500.0);
        let mut sprinting = leader;
        sprinting.sprinting = true;
        assert_eq!(
            follow_leader(&mut sprint, Some(&sprinting), true, false),
            FollowAct::Pursue
        );
        assert_eq!(sprint.urgency, 2);

        let mut busy = follower_at(1_000.0);
        busy.busy = true;
        assert_eq!(
            follow_leader(&mut busy, Some(&leader), true, false),
            FollowAct::Activity
        );

        let mut alongside = follower_at(300.0);
        alongside.mode = FOLLOWING;
        alongside.busy = true;
        assert_eq!(
            follow_leader(&mut alongside, Some(&leader), true, false),
            FollowAct::Pursue
        );
        assert_eq!(alongside.urgency, 0);

        let mut held = follower_at(500.0);
        held.mode = FOLLOWING;
        held.walking = true;
        held.planned = Vec3::new(100.0, 0.0, 0.0);
        assert_eq!(
            follow_leader(&mut held, Some(&leader), true, true),
            FollowAct::Pursue
        );
        assert!(!held.replan);
        assert_eq!(held.mode, FOLLOWING);
        assert_eq!(held.planned, Vec3::new(100.0, 0.0, 0.0));

        let mut moved = follower_at(500.0);
        moved.mode = FOLLOWING;
        moved.walking = true;
        moved.planned = Vec3::new(101.0, 0.0, 0.0);
        assert_eq!(
            follow_leader(&mut moved, Some(&leader), true, true),
            FollowAct::Pursue
        );
        assert!(moved.replan);
        assert_eq!(moved.planned, leader.place);
        assert_eq!(moved.stop_cm, 50);

        let mut hidden = follower_at(500.0);
        hidden.mode = FOLLOWING;
        assert_eq!(
            follow_leader(&mut hidden, Some(&leader), false, true),
            FollowAct::Stand
        );
        assert!(!hidden.replan);
    }

    #[test]
    fn a_sleeper_lies_dormant_and_its_tick_changes_nothing() {
        let mut sleeper = Sleeper {
            condition: Condition::WALKING,
            flags: StateFlags(StateFlags::SHOCK_STAGGER),
            statue: false,
            gait: 0,
            tick: IdleTick::Active,
            sighted: 0,
            wake_at: 0,
        };
        let mut drive = Drive::default();
        assert!(drive.tone_target() > 0.0);

        sleep(&mut sleeper, &mut drive);
        assert_eq!(sleeper.condition, Condition::DORMANT);
        assert_eq!(
            sleeper.flags.0,
            StateFlags::SHOCK_STAGGER | StateFlags::DORMANT
        );
        assert_eq!(drive.tone, 0.0);
        assert_eq!(drive.tone_target(), 0.0);
        assert!(sleeper.statue);
        assert_eq!(sleeper.gait, 2);
        assert_eq!(sleeper.tick, IdleTick::Sleep);

        let kept = sleeper;
        let tone = drive.tone;
        sleep_tick(&mut sleeper, &mut drive);
        assert_eq!(sleeper, kept);
        assert_eq!(drive.tone, tone);
        assert_eq!(drive.tone_target(), 0.0);

        sleep(&mut sleeper, &mut drive);
        assert_eq!(sleeper, kept);
        assert_eq!(drive.tone, 0.0);
    }

    fn limp() -> exa_physics::body::Ragdoll {
        use bevy::math::Affine3A;
        use bevy::prelude::Entity;
        use exa_physics::body::{PartSetup, Ragdoll};
        Ragdoll::new(&[PartSetup {
            bone: 11,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::ZERO),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.06),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        }])
    }

    #[test]
    fn a_dormant_sleeper_wakes_when_it_sees_a_contact() {
        use exa_physics::body::WAKE_COUNT;

        let mut sleeper = Sleeper {
            condition: Condition::WALKING,
            flags: StateFlags(0),
            statue: false,
            gait: 0,
            tick: IdleTick::Active,
            sighted: 0,
            wake_at: -1,
        };
        let mut drive = Drive::default();
        drive.pin_gain = 0.02;
        let mut roles = vec![SLEEP_ROLE, 3];
        let mut body = limp();
        body.tick(&[] as &[[Vec3; 3]]);
        assert!(body.countdown() < WAKE_COUNT);
        sleep(&mut sleeper, &mut drive);
        assert_eq!(sleeper.flags.0, StateFlags::DORMANT);

        assert!(!see_target(
            &mut sleeper,
            &mut drive,
            &mut body,
            &mut roles,
            false,
            2_500
        ));
        assert_eq!(sleeper.condition, Condition::DORMANT);
        assert!(sleeper.statue);
        assert_eq!(roles, vec![SLEEP_ROLE, 3]);
        assert!(body.countdown() < WAKE_COUNT);

        assert!(see_target(
            &mut sleeper,
            &mut drive,
            &mut body,
            &mut roles,
            true,
            2_500
        ));
        assert_eq!(sleeper.condition, Condition::DOWN);
        assert_eq!(sleeper.flags.0, StateFlags::DORMANT | StateFlags::WAKING);
        assert!(!sleeper.statue);
        assert_eq!(sleeper.gait, 0);
        assert_eq!(drive.tone, 0.0);
        assert_eq!(drive.pin_gain, 0.08);
        assert_eq!(sleeper.sighted, 1);
        assert_eq!(sleeper.wake_at, 2_500);
        assert_eq!(roles, vec![3]);
        assert_eq!(body.countdown(), WAKE_COUNT);

        let mut marked = Sleeper {
            condition: Condition::WALKING,
            flags: StateFlags(StateFlags::SHOCK_STAGGER),
            statue: false,
            gait: 0,
            tick: IdleTick::Active,
            sighted: 0,
            wake_at: 0,
        };
        let mut marked_drive = Drive::default();
        let mut marked_roles = vec![SLEEP_ROLE];
        let mut marked_body = limp();
        sleep(&mut marked, &mut marked_drive);
        assert!(!see_target(
            &mut marked,
            &mut marked_drive,
            &mut marked_body,
            &mut marked_roles,
            true,
            9
        ));
        assert_eq!(marked.condition, Condition::DORMANT);
        assert!(marked.statue);
        assert_eq!(marked_roles, vec![SLEEP_ROLE]);
        assert_eq!(marked.sighted, 0);
    }
}
