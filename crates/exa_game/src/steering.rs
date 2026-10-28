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
//!   two double-support keys), so the body finishes its stride. A key
//!   pressed for under 200 ms keeps its movement word until 200 ms after
//!   the press (`MoveWord`), so a tap still takes a step; a held key stops
//!   at once. Opposing keys cancel in the word (A with D, W with S). The
//!   pad's analog stick is its own word.
//! - **The cursor.** Its point on the plane 80 cm over the ground gives a
//!   direction once it is 50 cm or more away, which a second-order yaw
//!   follower smooths.
//! - **Steering.** Moving by the keys keeps the heading (A and D turn it
//!   ±0.025 rad a tick), the head looking 30 % toward the smoothed cursor;
//!   moving by the mouse takes the cursor's direction (a reversal beyond
//!   120° turns first); either walks free along the facing with a
//!   look-ahead 75 + 6·v² cm ahead (70 cm behind backwards), which the
//!   grid's line walk clamps before a wall (`exa_world::nav::query`),
//!   switching the body to walk to the clamped point and brake there.
//!   Shift sets the sprint latch: with the keys it is spent on a jog; the
//!   mouse beyond 3 m runs and keeps it. Standing, A or D turns a quarter,
//!   more than 1 s after any turn key; released, a free walk stops softly
//!   8·v + 20 cm ahead in three dimensions.
//! - **The spawn.** On its third behaviour tick (unless down) the
//!   character is moved onto the nearest walkable point of the grid.
//! - **Footwork.** In combat mode a step's target is the chest plus the
//!   raw key sum scaled by (fd4·√scale)/|v| in centimetres (a long step
//!   once the same keys have been held strictly over 200 ms while the
//!   crouch widened is below 0.1, both scaled by the legs' extension),
//!   clamped by the line walk from the chest's x/z at the ground. A side
//!   key weighs 0.8 of the facing. The step is taken only when the clamped
//!   target is over 400 cm² from the chest, in three dimensions.

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
/// The look-ahead: 75 cm + 6·v² ahead (v in cm a tick; 45 cm + 6·v² in
/// combat mode's walk), 70 cm behind backwards; a wall within 100 cm also
/// stops the run.
const LOOK_AHEAD: f32 = 75.0;
const COMBAT_LOOK_AHEAD: f32 = 45.0;
const LOOK_AHEAD_SPEED: f32 = 6.0;
const LOOK_BEHIND: f32 = -70.0;
const RUN_CUT_WITHIN: f32 = 100.0;
/// The behaviour tick on which the character is moved onto the grid's
/// nearest walkable point, unless the condition has [`SNAP_HELD`] (down,
/// getting up or dead).
const SNAP_TICK: u8 = 3;
const SNAP_HELD: u32 = 0x200;
/// A soft stop pulls a goal farther than 8·v + 20 cm in three dimensions.
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

/// The exploration movement word's bits, as the key reader builds them:
/// forward (W), left (A), back (S) and right (D), and the gamepad's analog
/// word. Opposing keys cancel: A with D, W with S.
pub(crate) const MOVE_FORWARD: u32 = 1;
pub(crate) const MOVE_LEFT: u32 = 2;
pub(crate) const MOVE_BACK: u32 = 4;
pub(crate) const MOVE_RIGHT: u32 = 8;
pub(crate) const MOVE_ANALOG: u32 = 0x8000;
/// The movement word's persistence (ms): a word the keys held for less
/// than this is kept until this long after the press, so a tap moves for a
/// step; the body's movement target is let go this long after the last
/// nonzero tick.
pub(crate) const TAP_MS: u64 = 200;
/// The crouch amount under which a held word counts as a tap (compared in
/// double precision against 0.1).
pub(crate) const TAP_CROUCH: f64 = 0.1;

/// A side key's share of the facing in the movement goal's direction.
pub(crate) const GOAL_SIDE: f32 = 0.8;
/// A tap's stride share: `1.5 − 0.3·encumbrance`, worked in double
/// precision.
pub(crate) const TAP_STRIDE: f64 = 1.5;
pub(crate) const TAP_STRIDE_ENC: f64 = 0.3;
/// The squared distance (cm²) past which the goal lets go of the movement
/// target: 400, the square of 20 cm.
pub(crate) const GOAL_RELEASE2: f32 = 400.0;

/// The direction the movement word asks for, in the body's frame: forward
/// is the facing, back its negation, left and right the side vector
/// `(facing.z, 0, −facing.x)` (the cross of up and the facing) at
/// [`GOAL_SIDE`]; no bit gives zero. The analog word is its own direction
/// and does not come through here.
///
/// Kept as a tested rule: Exarust's exploration request walks by the body's
/// look-ahead, not by this goal, so nothing calls it yet.
#[allow(dead_code)]
pub(crate) fn key_goal_direction(facing: Vec3, word: u32) -> Vec3 {
    let side = Vec3::new(facing.z, 0.0, -facing.x) * GOAL_SIDE;
    let mut dir = if word & MOVE_FORWARD != 0 {
        facing
    } else {
        Vec3::ZERO
    };
    if word & MOVE_LEFT != 0 {
        dir += side;
    }
    if word & MOVE_RIGHT != 0 {
        dir -= side;
    }
    if word & MOVE_BACK != 0 {
        dir -= facing;
    }
    dir
}

/// The stride (cm) a movement goal is laid at: the body's step times its
/// clock factor, and while the word is a tap `(1.5 − 0.3·encumbrance)`
/// times the step first (in double precision, narrowed once), as the
/// original lengthens a tapped step.
///
/// Kept as a tested rule with the same missing caller as
/// [`key_goal_direction`].
#[allow(dead_code)]
pub(crate) fn goal_stride(step_cm: f32, clock: f32, encumbrance: f32, tap: bool) -> f32 {
    if tap {
        let stride =
            ((TAP_STRIDE - f64::from(encumbrance) * TAP_STRIDE_ENC) * f64::from(step_cm)) as f32;
        stride * clock
    } else {
        step_cm * clock
    }
}

/// The step vector a movement word asks for (cm), the original's key
/// direction scaled by `(step·factor)/|dir|`: W is the facing, S back, A and
/// D the 0.8 side, the analog word its own stored direction; the height is
/// dropped, and a zero direction divides by zero as the original does.
///
/// Kept as a tested rule: the running request walks by the look-ahead, so
/// nothing calls it yet.
#[allow(dead_code)]
pub(crate) fn key_step(facing: Vec3, word: u32, analog: Vec3, step_cm: f32, factor: f32) -> Vec3 {
    let dir = if word == MOVE_ANALOG {
        analog.with_y(0.0)
    } else {
        key_goal_direction(facing, word)
    };
    let length = dir.length();
    dir * ((step_cm * 1.0 * factor) / length)
}

/// Whether a goal this far from the body (cm) lets go of the movement
/// target: the squared distance, summed in the original's order, strictly
/// over [`GOAL_RELEASE2`], so 20 cm does not and 20.1 cm does (a NaN never
/// does).
pub(crate) fn goal_released(offset: Vec3) -> bool {
    GOAL_RELEASE2 < (offset.x * offset.x + offset.y * offset.y) + offset.z * offset.z
}
/// The analog stick's squared length over which the analog word replaces
/// the keys (compared in double precision against 0.01).
pub(crate) const ANALOG_DEAD_ZONE: f64 = 0.01;
/// An analog tap needs the stick's squared length over this (double).
pub(crate) const ANALOG_TAP: f64 = 0.9;

/// The exploration movement word's persistence, as the behaviour's key
/// state machine keeps it: the word the movement request reads (`held`),
/// the tap flag and three clocks (when `held` was set, when the raw word
/// last changed or stayed zero, and the last tick it was nonzero). A word
/// pressed for less than [`TAP_MS`] is kept until [`TAP_MS`] after the
/// press, so a quick tap still moves; a word held longer clears as soon as
/// it is released.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct MoveWord {
    /// The word the request reads.
    pub(crate) held: u32,
    /// The word stayed the same past [`TAP_MS`] with the body not
    /// crouching.
    pub(crate) tap: bool,
    /// When `held` became nonzero (ms).
    down_ms: u64,
    /// When the raw word last changed, or last read zero (ms).
    change_ms: u64,
    /// The last tick the raw word was nonzero (ms).
    busy_ms: u64,
    /// The pad's analog stick last read as the analog word.
    pub(crate) analog: Vec2,
    /// Set on the tick the word has been zero for over [`TAP_MS`] since its
    /// last nonzero tick: the original lets go of the body's movement
    /// target then (that target is not modelled; the flag reports it).
    pub(crate) target_expired: bool,
}

impl MoveWord {
    /// The raw word from the keys `[forward, back, left, right]` (W, S, A,
    /// D), opposing keys cancelling as the reader's order does: A sets
    /// left, D cancels a left or sets right, W sets forward, S cancels a
    /// forward or sets back.
    pub(crate) fn of_keys(keys: [bool; 4]) -> u32 {
        let [forward, back, left, right] = keys;
        let mut word = if left { MOVE_LEFT } else { 0 };
        if right {
            word = if word == MOVE_LEFT { 0 } else { MOVE_RIGHT };
        }
        if forward {
            word |= MOVE_FORWARD;
        }
        if back {
            if word & MOVE_FORWARD != 0 {
                word &= !MOVE_FORWARD;
            } else {
                word |= MOVE_BACK;
            }
        }
        word
    }

    /// One tick at `now_ms`: the keys `[forward, back, left, right]`, the
    /// pad's analog stick (its squared length over [`ANALOG_DEAD_ZONE`]
    /// reads the analog word; a NaN fails the ordered test and reads the
    /// keys) and the body's crouch amount (0 to 1). Returns the word the
    /// request reads.
    pub(crate) fn tick(&mut self, now_ms: u64, keys: [bool; 4], analog: Vec2, crouch: f32) -> u32 {
        let squared = analog.x * analog.x + analog.y * analog.y;
        let word = if f64::from(squared) > ANALOG_DEAD_ZONE {
            self.analog = analog;
            MOVE_ANALOG
        } else {
            Self::of_keys(keys)
        };
        self.target_expired = false;
        if word != self.held {
            self.tap = false;
            self.change_ms = now_ms;
            if word != 0 {
                self.held = word;
                self.down_ms = now_ms;
                self.busy_ms = now_ms;
            } else if self.down_ms + TAP_MS < now_ms {
                self.held = 0;
            }
        } else if word != 0 {
            self.busy_ms = now_ms;
            if self.change_ms + TAP_MS < now_ms
                && f64::from(crouch) < TAP_CROUCH
                && (word != MOVE_ANALOG || f64::from(squared) > ANALOG_TAP)
            {
                self.tap = true;
            }
        } else {
            self.tap = false;
            self.change_ms = now_ms;
            if self.busy_ms + TAP_MS < now_ms {
                self.target_expired = true;
            }
        }
        self.held
    }

    /// The word as the request's four keys `[forward, back, left, right]`.
    pub(crate) fn keys(word: u32) -> [bool; 4] {
        [
            word & MOVE_FORWARD != 0,
            word & MOVE_BACK != 0,
            word & MOVE_LEFT != 0,
            word & MOVE_RIGHT != 0,
        ]
    }
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

/// What a grid query of the behaviour was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asked {
    /// The look-ahead of a walk.
    LookAhead,
    /// A soft stop's goal.
    SoftStop,
    /// A footwork step's target.
    Step,
}

/// The behaviour's last line walk on the grid, for the navigation overlay:
/// from where, toward what, where it ended and whether something blocked
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineAsked {
    pub(crate) asked: Asked,
    pub(crate) from: Vec3,
    pub(crate) to: Vec3,
    pub(crate) end: Vec3,
    pub(crate) blocked: bool,
}

/// A line walk: from where the character stands to a goal; the goal
/// clamped before the first obstacle and whether one was met.
pub(crate) type LineWalk<'a> = &'a dyn Fn(Vec3, Vec3) -> (Vec3, bool);

/// Walks `line` from `from` to `to` and records it as `asked`.
fn ask(line: LineWalk, asked: Asked, from: Vec3, to: Vec3) -> LineAsked {
    let (end, blocked) = line(from, to);
    LineAsked {
        asked,
        from,
        to,
        end,
        blocked,
    }
}

/// How far ahead the look-ahead lies (cm) at `speed` (cm a tick): 75 + 6·v²,
/// 45 + 6·v² for combat mode's walk, 70 cm behind backwards (out of combat
/// mode).
pub(crate) fn look_ahead_reach(speed: f32, backwards: bool, combat: bool) -> f32 {
    if combat {
        speed * speed * LOOK_AHEAD_SPEED + COMBAT_LOOK_AHEAD
    } else if backwards {
        LOOK_BEHIND
    } else {
        speed * speed * LOOK_AHEAD_SPEED + LOOK_AHEAD
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
    /// no quarter turns meanwhile. The hand sets it ([`crate::hand`]).
    pub(crate) holds_object: bool,
    /// Behaviour ticks counted toward the spawn snap.
    pub(crate) ticks: u8,
    /// The last line walk on the grid (the navigation overlay draws it).
    pub(crate) line: Option<LineAsked>,
}

impl Steering {
    /// The behaviour tick's spawn snap, as the original's: counting its
    /// first ticks, on the third (unless `condition` is down, getting up or
    /// dead) the move across the ground (m) from where the character
    /// stands (`standing`) to `nearest`, the grid's nearest walkable point
    /// to it; none otherwise.
    pub(crate) fn snap(
        &mut self,
        condition: Condition,
        standing: Vec3,
        nearest: &dyn Fn(Vec3) -> Vec3,
    ) -> Option<Vec3> {
        if self.ticks >= SNAP_TICK {
            return None;
        }
        self.ticks += 1;
        if self.ticks < SNAP_TICK || condition.has(SNAP_HELD) {
            return None;
        }
        Some((nearest(standing) - standing).with_y(0.0))
    }

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
    /// `body`; `line` is the grid's line walk.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn explore(
        &mut self,
        req: &mut Locomotion,
        body: &Locomotor,
        keys: &Keys,
        now_ms: u64,
        standing: Vec3,
        condition: Condition,
        line: LineWalk,
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
                self.line = Some(soft_stop(req, body, standing, line));
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
            let backwards = req.flags & (BACK | TURN_ABOUT) != 0;
            let reach = look_ahead_reach(body.speed, backwards, false);
            let asked = ask(
                line,
                Asked::LookAhead,
                standing,
                standing + facing * (reach / CM),
            );
            self.line = Some(asked);
            let (goal, blocked) = (asked.end, asked.blocked);
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

    /// The right button in combat mode, each behaviour tick: held, it asks
    /// the walk to the cursor however near the cursor is, walking within
    /// 3 m and running beyond (the original's combat branch takes the walk
    /// mode while the button is down and drops it at once on the release,
    /// and its cursor shows the chevrons meanwhile).
    pub(crate) fn combat_request(&mut self, mouse: bool) {
        if !mouse {
            return;
        }
        self.requested = true;
        self.mode = if self.raw.is_some_and(|(_, d)| d > CURSOR_RUN_FROM) {
            CursorMode::Run
        } else {
            CursorMode::Walk
        };
    }

    /// The right button's walk in combat mode, as the original's combat
    /// branch: the request keeps only its sprint latch and asks to move, the
    /// heading and the look take the cursor's direction, a cursor beyond 3 m
    /// runs and a nearer one walks unless the Run key's latch turns into the
    /// run; the walk is free. While no foot steps, the look-ahead
    /// `6·v² + 45` cm along the body's facing is cast, and a hit stops the
    /// free walk and, within 100 cm, the run and the sprint. The footwork
    /// then gives way to the walk. Returns whether the walk is asked (not
    /// while the cursor is within 50 cm, which has no direction here).
    pub(crate) fn combat_walk(
        &mut self,
        req: &mut Locomotion,
        body: &Locomotor,
        standing: Vec3,
        to: Vec3,
        line: LineWalk,
    ) -> bool {
        use flags::*;
        let flat = to.with_y(0.0);
        if flat.length() < CURSOR_MIN {
            return false;
        }
        req.heading = yaw_of(flat);
        req.look = req.heading;
        req.flags = (req.flags & SPRINT) | MOVE;
        req.flags = if self.mode == CursorMode::Run {
            req.flags | RUN
        } else if !req.has(SPRINT) {
            req.flags & !RUN
        } else {
            (req.flags & !SPRINT) | RUN
        };
        req.slow_turn = false;
        req.moving = true;
        req.free_walk = true;
        self.requested = true;
        if req.stepping.is_none() {
            let reach = look_ahead_reach(body.speed, false, true);
            let asked = ask(
                line,
                Asked::LookAhead,
                standing,
                standing + body.facing() * (reach / CM),
            );
            self.line = Some(asked);
            req.goal = asked.end;
            if asked.blocked {
                req.free_walk = false;
                if (asked.end - standing).length() * CM < RUN_CUT_WITHIN {
                    req.flags &= !(RUN | SPRINT);
                }
            }
        }
        true
    }

    /// The combat branch's part in the locomotion until the combat
    /// steering is the original's: no slow turn, and a body still moving
    /// stops softly.
    pub(crate) fn fight(
        &mut self,
        req: &mut Locomotion,
        body: &Locomotor,
        standing: Vec3,
        line: LineWalk,
    ) {
        req.slow_turn = false;
        self.requested = false;
        if req.moving {
            self.line = Some(soft_stop(req, body, standing, line));
        }
    }

    /// A footwork step's target, as the original's: `target` (m) is the
    /// chest plus the scaled key vector, the line walk starts from the
    /// chest's x/z at the ground's height, and the step is taken only when
    /// the clamped target's squared distance from the chest, in centimetres
    /// and three dimensions, is over 400. An unordered distance is none.
    pub(crate) fn step_target(
        &mut self,
        chest: Vec3,
        ground: f32,
        target: Vec3,
        line: LineWalk,
    ) -> Option<Vec3> {
        let from = Vec3::new(chest.x, ground, chest.z);
        let asked = ask(line, Asked::Step, from, target);
        self.line = Some(asked);
        let centimetres = (asked.end - chest) * CM;
        goal_released(centimetres).then_some(asked.end)
    }
}

/// The look eased a twentieth of the way toward `wanted` (a unit direction,
/// or a blend of two), then normalised, as a yaw.
fn ease_look(look: f32, wanted: Vec3) -> f32 {
    let from = forward(look);
    yaw_of((from + (wanted - from) * LOOK_EASE).normalize_or(wanted))
}

/// The original's soft stop: free walk and run off, stopping on; a goal
/// farther than 8·v + 20 cm in three dimensions is brought back along the
/// facing, then clamped by the line walk (whose answer it records).
pub(crate) fn soft_stop(
    req: &mut Locomotion,
    body: &Locomotor,
    standing: Vec3,
    line: LineWalk,
) -> LineAsked {
    req.free_walk = false;
    req.flags = (req.flags & !flags::RUN) | flags::STOPPING;
    let centimetres = (req.goal - standing) * CM;
    let reach = body.speed * SOFT_STOP_SPEED + SOFT_STOP_MARGIN;
    if centimetres.length_squared() > reach * reach {
        req.goal = standing + body.facing() * (reach / CM);
    }
    let asked = ask(line, Asked::SoftStop, standing, req.goal);
    req.goal = asked.end;
    asked
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
