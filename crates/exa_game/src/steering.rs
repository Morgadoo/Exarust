//! The player's behaviour in exploration, as the original's: what the keys,
//! the right mouse button and the cursor ask of the body's locomotion
//! request (`exa_physics::body::locomotion`) each behaviour tick. The body
//! tick's movement decision then walks, brakes, stops and turns the body
//! (`crate::stages::movement`). Rules: `docs/PLAYER.md` (Movement).
//!
//! - **The request.** W, or the right button over the view, asks to move;
//!   S asks to move backwards (braking first, and turning the brake into a
//!   backpedal, when faster than 1 cm a tick). Let go while the body walks,
//!   the request holds until the playing walk's key segment is 0 or 4 (its
//!   two double-support keys), so the body finishes its stride.
//! - **The cursor.** Its point on the plane 80 cm over the ground gives a
//!   direction once it is 50 cm or more away, which a second-order yaw
//!   follower smooths.
//! - **Steering.** Moving by the keys keeps the heading (A and D turn it
//!   ±0.025 rad a tick), the head looking 30 % toward the smoothed cursor;
//!   moving by the mouse takes the cursor's direction (a reversal beyond
//!   120° turns first); either walks free along the facing with a
//!   look-ahead 75 + 6·v² cm ahead (70 cm behind backwards), which a wall
//!   clamps, switching the body to walk to the clamped point and brake
//!   there. Shift sets the sprint latch: with the keys it is spent on a
//!   jog; the mouse beyond 3 m runs and keeps it. Standing, A or D turns a
//!   quarter, more than 1 s after any turn key; released, a free walk stops
//!   softly 8·v + 20 cm ahead.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::Condition;
use exa_physics::body::locomotion::{
    Locomotion, Locomotor, flags, forward, turn_between, wrap, yaw_of,
};

/// A and D turn the heading this much a tick while moving (rad).
pub(crate) const STEER: f32 = 0.025;
/// Standing, A or D turns a quarter once more than this (ms of the game
/// clock) has passed since the last turn key.
const QUARTER_AFTER_MS: u64 = 1000;
/// S slower than this (cm a tick) backpedals at once; faster it brakes
/// first and turns the brake into a backpedal.
const BACKPEDAL_BELOW: f32 = 1.0;
/// The right button walks to a cursor this close (m) and runs to one
/// further away.
pub(crate) const CURSOR_RUN_FROM: f32 = 3.0;
/// A cursor closer than this (m) gives no direction.
const CURSOR_MIN: f32 = 0.5;
/// The look-ahead: 75 cm + 6·v² ahead (v in cm a tick), 70 cm behind
/// backwards; a wall within 100 cm also stops the run.
const LOOK_AHEAD: f32 = 75.0;
const LOOK_AHEAD_SPEED: f32 = 6.0;
const LOOK_BEHIND: f32 = -70.0;
const RUN_CUT_WITHIN: f32 = 100.0;
/// A soft stop's goal lies at most 8·v + 20 cm ahead.
const SOFT_STOP_SPEED: f32 = 8.0;
const SOFT_STOP_MARGIN: f32 = 20.0;
/// By the mouse, a wanted direction this far behind the facing (its dot
/// product, beyond 120°) turns the body before it walks.
const REVERSAL: f32 = -0.5;
/// The head's look eases this share of the way a tick standing (and in the
/// reversal), toward the heading moved 40 % toward the smoothed cursor;
/// moving by the keys it looks 30 % toward it at once. Standing, the
/// smoothed cursor direction is mixed 60 % with the facing every tick.
const LOOK_EASE: f32 = 0.05;
const STANDING_LOOK: f32 = 0.4;
const WALKING_LOOK: f32 = 0.3;
const STANDING_CURSOR: f32 = 0.6;
/// Key segments of the walk at which a released request lets go: its two
/// double-support keys.
const RELEASE_SEGMENTS: [usize; 2] = [0, 4];
/// The cursor follower: each tick the error (clamped to ±π/2) adds
/// 0.015708 of itself to the turn speed, which keeps 0.7; a speed under
/// 1e-5 rad is no turn.
const FOLLOW_GAIN: f32 = 0.015_707_964;
const FOLLOW_KEEP: f32 = 0.7;
const FOLLOW_STILL: f64 = 1e-5;
/// No steering while the condition has any of these bits: down, getting up
/// or dead (0x200), switching held items (4) and the original's 0x40000.
const NO_STEERING: u32 = 0x40204;
/// No movement request while down or dead.
const NO_REQUEST: u32 = 0xA00;
/// Centimetres per metre.
const CM: f32 = 100.0;

/// The keys and buttons a behaviour tick reads.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Keys {
    pub(crate) w: bool,
    pub(crate) a: bool,
    pub(crate) s: bool,
    pub(crate) d: bool,
    pub(crate) shift: bool,
    /// The right button is held over the view.
    pub(crate) mouse: bool,
}

/// What the cursor asks of the movement while it is not the keys: the
/// walk chevron within 3 m, the run chevron beyond.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CursorMode {
    #[default]
    Other,
    Walk,
    Run,
}

/// The original's cursor heading smoother: a second-order follower of the
/// cursor's yaw.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CursorFollower {
    /// The yaw it follows, unwrapped to within half a turn of `yaw`.
    target: f32,
    /// The smoothed yaw and its turn speed (rad a tick).
    yaw: f32,
    speed: f32,
}

impl CursorFollower {
    /// A follower resting at `yaw`.
    pub(crate) fn at(yaw: f32) -> Self {
        Self {
            target: yaw,
            yaw,
            speed: 0.0,
        }
    }

    /// Follows `yaw` from now on.
    pub(crate) fn follow(&mut self, yaw: f32) {
        self.target = self.yaw + turn_between(self.yaw, yaw);
    }

    /// One tick: e = clamp(target − yaw, ±π/2), speed = (speed + 0.015708·e)
    /// × 0.7, the yaw moving by the speed unless it is under 1e-5.
    pub(crate) fn tick(&mut self) {
        let error = turn_between(self.yaw, self.target)
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        self.speed = (error * FOLLOW_GAIN + self.speed) * FOLLOW_KEEP;
        if f64::from(self.speed.abs()) > FOLLOW_STILL {
            self.yaw = wrap(self.yaw + self.speed);
        }
    }

    /// The smoothed yaw.
    pub(crate) fn yaw(&self) -> f32 {
        self.yaw
    }
}

/// What the player's behaviour keeps between ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Steering {
    /// A move is requested: by a key or the mouse, and after their release
    /// until the walk reaches a release segment.
    pub(crate) requested: bool,
    /// The game clock (ms) of the last turn key: a quarter turn or A or D
    /// steering a walk.
    pub(crate) turn_key_ms: u64,
    /// The cursor's raw direction (a yaw) and distance (m), once it has
    /// been 50 cm or more away.
    pub(crate) raw: Option<(f32, f32)>,
    /// The smoothed cursor direction, once started.
    pub(crate) follower: Option<CursorFollower>,
    /// What the cursor asks this tick.
    pub(crate) mode: CursorMode,
    /// The left button holds an object (a lever, a door, a dragged item):
    /// no quarter turns meanwhile. Nothing sets it until the player can
    /// handle the world's objects.
    pub(crate) holds_object: bool,
}

impl Steering {
    /// The behaviour tick's common part: Shift held sets the sprint latch,
    /// released clears it, every tick.
    pub(crate) fn latch(req: &mut Locomotion, keys: &Keys) {
        if keys.shift {
            req.flags |= flags::SPRINT;
        } else {
            req.flags &= !flags::SPRINT;
        }
    }

    /// The movement request, as the original's (per frame there, per
    /// behaviour tick here): nothing while down or dead; W or the mouse
    /// asks to move, S too, backwards (braking first above 1 cm a tick and
    /// turning about once slow); otherwise moving clears backwards. The
    /// keys set the heading-driven bit, the mouse clears it. Let go while
    /// the body moves, the request holds (heading-driven) until the walk's
    /// key segment `segment` is 0 or 4 (or no walk plays), which also
    /// clears backwards. Then the cursor's mode: the mouse's request walks
    /// to a cursor within 3 m and runs beyond.
    pub(crate) fn request(
        &mut self,
        req: &mut Locomotion,
        body: &Locomotor,
        keys: &Keys,
        segment: Option<usize>,
        condition: Condition,
    ) {
        use flags::*;
        if condition.has(NO_REQUEST) {
            return;
        }
        let mut wanted = keys.w || keys.mouse;
        if keys.s {
            wanted = true;
            if !req.has(BACK) {
                req.flags |= if body.speed <= BACKPEDAL_BELOW {
                    BACK
                } else {
                    BRAKE | TURN_ABOUT
                };
            }
        } else if wanted {
            req.flags &= !(BACK | TURN_ABOUT);
        }
        if wanted {
            if keys.mouse {
                req.flags &= !KEYS;
            } else {
                req.flags |= KEYS;
            }
            self.requested = true;
        } else if !req.moving {
            self.requested = false;
        } else if segment.is_none_or(|s| RELEASE_SEGMENTS.contains(&s)) {
            self.requested = false;
            req.flags &= !(BACK | TURN_ABOUT);
        } else {
            req.flags |= KEYS;
        }
        self.mode = if !self.requested || req.has(KEYS) {
            CursorMode::Other
        } else if self.raw.is_some_and(|(_, d)| d > CURSOR_RUN_FROM) {
            CursorMode::Run
        } else {
            CursorMode::Walk
        };
    }

    /// The cursor, each behaviour tick: its point `to` (from where the
    /// character stands, across the ground) gives its raw direction and
    /// distance once 50 cm or more away, which the follower takes as its
    /// target; the follower ticks, and its yaw is the request's smoothed
    /// cursor direction. Down (`down`), the raw direction and the look are
    /// the heading.
    pub(crate) fn aim(&mut self, req: &mut Locomotion, to: Option<Vec3>, down: bool) {
        let follower = self
            .follower
            .get_or_insert_with(|| CursorFollower::at(req.heading));
        if down {
            self.raw = Some((req.heading, self.raw.map_or(0.0, |(_, d)| d)));
            req.look = req.heading;
            follower.follow(req.heading);
        } else if let Some(to) = to.map(|t| t.with_y(0.0))
            && to.length() >= CURSOR_MIN
        {
            let yaw = yaw_of(to);
            self.raw = Some((yaw, to.length()));
            follower.follow(yaw);
        }
        follower.tick();
        req.cursor = follower.yaw();
        if let Some((yaw, _)) = self.raw {
            req.desired = yaw;
        }
    }

    /// The exploration steering, one behaviour tick at `now_ms` (the game
    /// clock), for a body standing at `standing` whose locomotion is
    /// `body`; `line` walks a straight line from where the character stands
    /// to a goal and returns the goal clamped before the first obstacle and
    /// whether one was met.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn explore(
        &mut self,
        req: &mut Locomotion,
        body: &Locomotor,
        keys: &Keys,
        now_ms: u64,
        standing: Vec3,
        condition: Condition,
        line: &dyn Fn(Vec3, Vec3) -> (Vec3, bool),
    ) {
        use flags::*;
        req.slow_turn = true;
        req.stepping = None;
        if condition.has(NO_STEERING) {
            return;
        }
        let facing = body.facing();
        let cursor = forward(req.cursor);
        if !self.requested {
            if !req.moving {
                let turn = keys.a || keys.d;
                if !self.holds_object && turn && self.turn_key_ms + QUARTER_AFTER_MS < now_ms {
                    let quarter = std::f32::consts::FRAC_PI_2;
                    let mut heading = req.heading;
                    if keys.a {
                        heading = wrap(heading + quarter);
                    }
                    if keys.d {
                        heading = wrap(heading - quarter);
                    }
                    req.heading = heading;
                    self.turn_key_ms = now_ms;
                }
                let mixed = (facing + (cursor - facing) * STANDING_CURSOR).normalize_or(facing);
                req.cursor = yaw_of(mixed);
                // Toward the heading moved 40 % toward the cursor, as it is
                // (the original eases toward the lerp unnormalised).
                let heading = forward(req.heading);
                req.look = ease_look(req.look, heading + (mixed - heading) * STANDING_LOOK);
            } else if req.free_walk {
                soft_stop(req, body, standing, line);
            } else {
                req.flags &= !RUN;
            }
            return;
        }
        if self.mode == CursorMode::Run {
            req.slow_turn = false;
        }
        let raw = forward(req.desired);
        let wanted = if facing.dot(raw) > cursor.dot(raw) {
            facing
        } else {
            cursor
        };
        if !req.has(KEYS) && !req.free_walk && facing.dot(wanted) <= REVERSAL {
            req.heading = yaw_of(wanted);
            req.look = ease_look(req.look, wanted);
            return;
        }
        if !req.has(KEYS) {
            req.heading = yaw_of(wanted);
            req.look = req.heading;
        } else {
            if keys.a || keys.d {
                let mut heading = req.heading;
                if keys.a {
                    heading = wrap(heading + STEER);
                }
                if keys.d {
                    heading = wrap(heading - STEER);
                }
                req.heading = heading;
                self.turn_key_ms = now_ms;
            }
            let heading = forward(req.heading);
            req.look = yaw_of((heading + (cursor - heading) * WALKING_LOOK).normalize_or(heading));
        }
        req.free_walk = true;
        req.flags &= !STOPPING;
        req.flags = if self.mode == CursorMode::Run {
            req.flags | RUN
        } else if !req.has(SPRINT) {
            req.flags & !RUN
        } else {
            (req.flags & !SPRINT) | RUN
        };
        req.moving = true;
        let mut keeps_brake = false;
        if req.stepping.is_none() {
            let reach = if req.flags & (BACK | TURN_ABOUT) != 0 {
                LOOK_BEHIND
            } else {
                body.speed * body.speed * LOOK_AHEAD_SPEED + LOOK_AHEAD
            };
            let (goal, blocked) = line(standing, standing + facing * (reach / CM));
            req.goal = goal;
            req.free_walk = !blocked;
            if blocked && (goal - standing).with_y(0.0).length() * CM < RUN_CUT_WITHIN {
                req.flags &= !(RUN | SPRINT);
                keeps_brake = true;
            }
        }
        if !keeps_brake && !req.has(TURN_ABOUT) {
            req.flags &= !BRAKE;
        }
    }

    /// The combat branch's part in the locomotion until the combat
    /// steering is the original's: no slow turn, and a body still moving
    /// stops softly.
    pub(crate) fn fight(
        &mut self,
        req: &mut Locomotion,
        body: &Locomotor,
        standing: Vec3,
        line: &dyn Fn(Vec3, Vec3) -> (Vec3, bool),
    ) {
        req.slow_turn = false;
        self.requested = false;
        if req.moving {
            soft_stop(req, body, standing, line);
        }
    }
}

/// The look eased a twentieth of the way toward `wanted` (a unit direction,
/// or a blend of two), then normalised, as a yaw.
fn ease_look(look: f32, wanted: Vec3) -> f32 {
    let from = forward(look);
    yaw_of((from + (wanted - from) * LOOK_EASE).normalize_or(wanted))
}

/// The original's soft stop: free walk and run off, stopping on; the goal
/// brought within 8·v + 20 cm ahead of where the character stands (along
/// its facing), then clamped by the line walk.
pub(crate) fn soft_stop(
    req: &mut Locomotion,
    body: &Locomotor,
    standing: Vec3,
    line: &dyn Fn(Vec3, Vec3) -> (Vec3, bool),
) {
    req.free_walk = false;
    req.flags = (req.flags & !flags::RUN) | flags::STOPPING;
    let most = (body.speed * SOFT_STOP_SPEED + SOFT_STOP_MARGIN) / CM;
    if (req.goal - standing).with_y(0.0).length() > most {
        req.goal = standing + body.facing() * most;
    }
    req.goal = line(standing, req.goal).0;
}

/// The key segment of the clip `anim` plays (as the original's sampler
/// counts them): 0 before the first key and, looping, past the last; else
/// the first key at or after the clock (less its blend).
pub(crate) fn key_segment(anim: &Animator, motions: &Motions) -> Option<usize> {
    let keys = &motions.clip(anim, &anim.action)?.keys;
    let (first, last) = (keys.first()?.time, keys.last()?.time);
    let lead = anim.time - anim.blend;
    Some(if lead <= first || (lead >= last && anim.looping) {
        0
    } else {
        (1..keys.len())
            .find(|&k| lead <= keys[k].time)
            .unwrap_or(keys.len() - 1)
    })
}

#[cfg(test)]
#[path = "steering_test.rs"]
mod tests;
