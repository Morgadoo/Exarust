//! How an NPC turns, looks and walks: the AI's own heading, look and gaze
//! fields, and the routines its idle and combat states call to move the
//! body.
//!
//! - **Gaze.** The AI keeps a look yaw, a head offset from it and the
//!   offset it is heading for: the head swings toward a point within a
//!   quarter turn of the look, easing its offset by 0.02 a tick, and the
//!   body's look direction is that offset added to the look yaw. A second
//!   routine eases the body's *whole* look toward the AI's heading: the gap
//!   is held within a quarter turn, the sway follows
//!   `(0.0062831854·gap + sway)·0.7`, and the look moves by the sway while
//!   it is over 1e-5; the body's look is written only when its gap is over
//!   the caller's threshold in degrees (30° at the end of the tick).
//! - **Idle look.** Out of combat the AI turns the body's look toward its
//!   look yaw by 0.1 of the gap (held within a quarter turn), smooths the
//!   pitch toward the stored pitch by 1 %, and then eases the body's look
//!   smoothing direction toward the body's facing by 0.4 of the gap
//!   (0.64 beyond 1.6 rad).
//! - **Walking.** [`walk`] is the step/combat walk driver: it follows the
//!   path, aims the body's look at the step goal, holds the goal within
//!   150 cm when it lies far to the side, chooses the walk/run flags from
//!   the facing and the ground, and looks the head at the foe. A body in
//!   the way stops the walk ([`stop_walk`]).
//! - **Turning.** Out of combat [`turn_towards`] sways the body's look
//!   around its heading: a draw every four seconds sets an offset of ±1.8
//!   rad and the look eases toward it by 0.09 − 0.06·encumbrance.
//! - **Alert.** [`alert_look`] sweeps a panicking AI's look: one draw in
//!   300 pans ±π about the body every 3 s (the next within 1 s), one draw
//!   in 75 pans ±π/2 every 700 ms; the head follows by 0.05 and the body
//!   by 0.4 of the gap (0.64 beyond 1.6 rad). A foe is looked at instead.
//!
//! The original keeps all these points relative to the scene node and adds
//! the node origin where it weighs them; Exarust keeps one world frame (cm).
//! The routines work on plain numbers, so a caller can run them without a
//! body; [`crate::brain::ai_tick`] only turns an idle character's model and
//! leaves the head look to the body's locomotion, which the running game
//! does not tick for an NPC.
//!
//! Evidence: the original's head-look, walk, turn, settle, aim, snap, stop,
//! alert and steer routines, read from the instructions (sums and products
//! in their order, comparisons widened to double where the original widens
//! them).

use bevy::math::Vec3;

use crate::combatant::asin_fast;

/// Centimetres per metre: the routines work in the original's units.
pub const CM: f32 = 100.0;
/// The f32 full turn.
pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
/// A quarter turn (the engine's f32 π/2).
pub const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
/// The factor 0.0062831854 the look sway's rate uses (a full turn over
/// 1000).
pub const SWAY_RATE: f32 = f32::from_bits(0x3bcde32e);
/// A degree in radians.
pub const DEGREE: f32 = f32::from_bits(0x3c8e_fa35);
/// The look is written once the gap is over this many degrees.
pub const LOOK_WRITE_DEGREES: f32 = 30.0;
/// The goal is held within this of the body when it lies to the side (cm).
pub const GOAL_HOLD_CM: f32 = 150.0;
/// The walk flag bit: running.
pub const RUN: u32 = 0x4;
/// The walk flag bit: the sprint latch.
pub const SPRINT: u32 = 0x8;
/// The walk flag bit: moving.
pub const MOVE: u32 = 0x1;
/// The walk flag bit: stopping.
pub const STOPPING: u32 = 0x20;
/// The body's facing dot above which it walks its goal directly, past 100 cm
/// of goal.
pub const FAR_GOAL: f32 = 0.7;
/// The gap above which a body's look turns at the larger rate (rad).
pub const WIDE_TURN: f64 = 1.6;
/// The two rates the look smoothing uses for its gap to the facing.
pub const NARROW_RATE: f64 = 0.4;
pub const WIDE_RATE: f64 = 0.64;
/// The pitch target the idle look eases toward is kept 1 % a tick.
pub const PITCH_EASE: f64 = 0.01;
/// The steer line's reach (cm) and the squared slack at its end (60 cm).
pub const STEER_REACH_CM: f32 = 120.0;
pub const STEER_SLACK_CM2: f32 = 3600.0;
/// A body's way share ignores offsets shorter than this, squared (cm²).
pub const WAY_MIN_CM2: f32 = 1.0;
/// The way share's reach margin (cm).
pub const WAY_MARGIN_CM: f32 = 100.0;
/// The alert look's panic floor, and the panic over which it marks the body
/// in combat.
pub const PANIC_FLOOR: f32 = 0.2;
pub const PANIC_MARK: f32 = 1.0;
/// The alert look's pan timers (ms) and the draws' divisors.
pub const WIDE_EVERY_MS: u32 = 3000;
pub const NARROW_EVERY_MS: u32 = 700;
pub const WIDE_ODDS: u64 = 300;
pub const NARROW_ODDS: u64 = 75;
/// The out-of-combat sway's span (rad) and its timer (ms).
pub const SWAY_SPAN: f32 = 3.6;
pub const SWAY_EVERY_MS: u32 = 4000;
/// The stop's pull-back: 8·crouch + 20 cm.
pub const STOP_CROUCH: f32 = 8.0;
pub const STOP_BASE_CM: f32 = 20.0;

/// The signed gap from `b` to `a`, wrapped once into ±π: the engine's angle
/// helper compares in double precision against the double π and steps by
/// the f32 full turn.
pub fn gap(a: f32, b: f32) -> f32 {
    let d = a - b;
    if f64::from(d) < -std::f64::consts::PI {
        d + TWO_PI
    } else if f64::from(d) > std::f64::consts::PI {
        d - TWO_PI
    } else {
        d
    }
}

/// The engine's sign: 1 or −1 by the sign bit, 0 for zero (NaN keeps its
/// sign bit's 1).
fn sign(x: f32) -> f32 {
    if x == 0.0 { 0.0 } else { 1.0_f32.copysign(x) }
}

/// The yaw of a direction along the ground (0 toward +Z, positive toward
/// +X), 0 for a flat offset shorter than 1e-8 (the engine's yaw helper).
pub fn yaw_of(d: Vec3) -> f32 {
    let flat = d.z * d.z + d.x * d.x;
    if flat < 1e-8 {
        return 0.0;
    }
    let c = d.z / flat.sqrt();
    let a = if c > -1.0 {
        if c <= 1.0 { c.acos() } else { 0.0 }
    } else {
        std::f32::consts::PI
    };
    if d.x < 0.0 { -a } else { a }
}

/// The unit direction of a yaw along the ground.
pub fn dir_of(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

/// A pitch and yaw as the engine builds a direction: `(cos p · sin y,
/// −sin p, cos p · cos y)`.
pub fn pitched(pitch: f32, yaw: f32) -> Vec3 {
    Vec3::new(
        pitch.cos() * yaw.sin(),
        -pitch.sin(),
        pitch.cos() * yaw.cos(),
    )
}

/// The engine's normalise: the length squared is `(y² + 0) + (x² + z²)` and
/// zero stays zero.
pub fn unit(v: Vec3) -> Vec3 {
    let sum = (v.y * v.y + 0.0) + (v.x * v.x + v.z * v.z);
    if sum == 0.0 {
        Vec3::ZERO
    } else {
        let s = 1.0 / sum.sqrt();
        Vec3::new(v.x * s, v.y * s, v.z * s)
    }
}

/// The AI's look state: the look yaw, the head's offset from it and where
/// that offset is heading, the sway rate, and the pitch the idle look eases
/// toward.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gaze {
    /// The look yaw (`+0x8d0`; the body's look yaw in combat).
    pub look: f32,
    /// The head's offset from the look (`+0x8d4` in combat; a target yaw
    /// out of combat).
    pub offset: f32,
    /// Where the offset is heading (`+0x8dc`).
    pub target_offset: f32,
    /// The sway rate (`+0x8e4`).
    pub sway: f32,
    /// The pitch the idle look's pitch eases toward (`+0x8d8`).
    pub pitch: f32,
}

impl Gaze {
    /// A gaze looking along `yaw`.
    pub fn at(yaw: f32) -> Self {
        Self {
            look: yaw,
            offset: yaw,
            target_offset: yaw,
            sway: 0.0,
            pitch: 0.0,
        }
    }
}

/// One tick of the body's head look toward `point_cm`, from the body's
/// position: the point's yaw less the look is the offset's target; within a
/// quarter turn (a NaN passes) the offset eases 0.02 toward it, wraps once,
/// and the look direction is the look yaw plus the offset, wrapped once.
/// Returns the direction for the body's look, or none when the point lies
/// behind the AI.
pub fn look_at(gaze: &mut Gaze, body_cm: Vec3, point_cm: Vec3) -> Option<Vec3> {
    let target = gap(yaw_of(point_cm - body_cm), gaze.look);
    gaze.target_offset = target;
    if target.abs() > HALF_PI {
        return None;
    }
    gaze.offset = (f64::from(target - gaze.offset) * 0.02 + f64::from(gaze.offset)) as f32;
    gaze.offset = gap(gaze.offset, 0.0);
    Some(dir_of(gap(gaze.look + gaze.offset, 0.0)))
}

/// One tick of the body's whole look toward `heading`: the gap to the look
/// is held within a quarter turn, the sway follows
/// `(0.0062831854·gap + sway)·0.7`, the look moves by the sway while its
/// size is over 1e-5 (wrapped into [0, 2π)), and the direction is written
/// when the gap from `body_look_yaw` is over `degrees`. Returns the
/// direction to write, or none.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn smooth_look(
    gaze: &mut Gaze,
    heading: f32,
    body_look_yaw: f32,
    degrees: f32,
) -> Option<Vec3> {
    let mut d = gap(heading, gaze.look);
    if d.abs() > HALF_PI {
        d = sign(d) * HALF_PI;
    }
    gaze.sway = (d * SWAY_RATE + gaze.sway) * 0.7;
    if f64::from(gaze.sway.abs()) > 1e-5 {
        gaze.look += gaze.sway;
        if gaze.look >= 0.0 {
            if gaze.look > TWO_PI {
                gaze.look -= TWO_PI;
            }
        } else {
            gaze.look += TWO_PI;
        }
    }
    let d = gap(body_look_yaw, gaze.look);
    if !(degrees * DEGREE < d.abs()) {
        return None;
    }
    Some(dir_of(gaze.look))
}

/// The body's idle look: its direction and pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Looking {
    /// The body's look direction (unit).
    pub look: Vec3,
}

/// One tick of the idle look: with no running look target the body's look
/// is turned toward `gaze.offset` by 0.1 of the gap (held within a quarter
/// turn), its pitch eases 1 % toward `gaze.pitch`, and the direction is
/// built from them; with a look target still running nothing is written.
/// Returns the direction to write and the yaw the look smoothing aims at.
pub fn idle_look(gaze: &Gaze, at: Looking, look_target_running: bool) -> (Option<Vec3>, f32) {
    let yaw = yaw_of(at.look);
    if look_target_running {
        return (None, yaw);
    }
    let mut d = gap(gaze.offset, yaw);
    if d.abs() > HALF_PI {
        d = sign(d) * HALF_PI;
    }
    let yaw = gap(yaw + (f64::from(d) * 0.1) as f32, 0.0);
    let pitch = asin_fast(-at.look.y);
    let smooth = (f64::from(gaze.pitch - pitch) * PITCH_EASE + f64::from(pitch)) as f32;
    (Some(pitched(smooth, yaw)), yaw)
}

/// The look smoothing's direction toward the body's facing from the look
/// yaw: 0.4 of the gap, 0.64 of its sign beyond 1.6 rad.
pub fn look_smoothing(body_heading_yaw: f32, look_yaw: f32) -> f32 {
    let d = gap(look_yaw, body_heading_yaw);
    let turn = if f64::from(d.abs()) > WIDE_TURN {
        (f64::from(sign(d)) * WIDE_RATE) as f32
    } else {
        (f64::from(d) * NARROW_RATE) as f32
    };
    gap(body_heading_yaw + turn, 0.0)
}

/// Whether the walk toward `to_cm` from `from_cm` along `direction` can be
/// steered within 120 cm: the projection of the offset on the direction
/// (over 120 cm) clamps to 1; the point reached must lie within 60 cm of
/// `to_cm`. A projection of 0 or less fails.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn steer_line(from_cm: Vec3, direction: Vec3, to_cm: Vec3) -> bool {
    let d = to_cm - from_cm;
    let projected = (d.x * direction.x + d.z * direction.z) * (1.0 / STEER_REACH_CM);
    if !(0.0 < projected) {
        return false;
    }
    let projected = if 1.0 < projected { 1.0 } else { projected };
    let reached = from_cm + direction * (projected * STEER_REACH_CM);
    let back = to_cm - reached;
    back.x * back.x + back.z * back.z < STEER_SLACK_CM2
}

/// A body of the AI's nearby list, as the way tests read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyAt {
    pub id: usize,
    pub position_cm: Vec3,
    pub dead: bool,
}

/// The nearest body in front of `from_cm` along `facing` within
/// `reach_cm + 100`: it must be at least 1 cm² away and its offset's dot
/// with the facing must not be negative; the largest `dot / |offset|²`
/// wins. Returns `1 − share` (1 when nothing is in front).
pub fn way_share(from_cm: Vec3, facing: Vec3, bodies: &[BodyAt], reach_cm: f32) -> f32 {
    let mut best = 0.0f32;
    for body in bodies {
        if body.dead {
            continue;
        }
        let d = body.position_cm - from_cm;
        let d2 = d.x * d.x + d.y * d.y + d.z * d.z;
        let limit = reach_cm + WAY_MARGIN_CM;
        if d2 > limit * limit || d2 < WAY_MIN_CM2 {
            continue;
        }
        let dot = d.z * facing.z + d.y * facing.y + d.x * facing.x;
        if dot < 0.0 {
            continue;
        }
        let share = dot / d2;
        if best < share {
            best = share;
        }
    }
    1.0 - best
}

/// The body in the way of the walk: each nearby live body's point is moved
/// 20 cm along the AI's look and the steer line from the AI along the walk
/// direction is tested; the first that can be steered to is returned.
pub fn body_in_way(from_cm: Vec3, direction: Vec3, look: Vec3, bodies: &[BodyAt]) -> Option<usize> {
    for body in bodies {
        if body.dead {
            continue;
        }
        let moved = body.position_cm + Vec3::new(look.x, 0.0, look.z) * 20.0;
        if steer_line(from_cm, direction, moved) {
            return Some(body.id);
        }
    }
    None
}

/// The body's walk request as the step/combat walk driver writes it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Walk {
    /// The step goal (cm).
    pub goal_cm: Vec3,
    /// The body's look direction (unit).
    pub look: Vec3,
    /// The walk flags ([`MOVE`], [`RUN`], [`SPRINT`]).
    pub flags: u32,
    /// The body is steering its own movement (`+0x12ea`).
    pub steering: bool,
    /// The body's look smoothing direction (unit).
    pub look_smooth: Vec3,
}

/// What [`walk`] reads of the body and its path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walking {
    pub body_cm: Vec3,
    /// The body's facing (unit).
    pub facing: Vec3,
    /// The body's look direction (unit).
    pub look: Vec3,
    /// The body's look smoothing direction (unit).
    pub look_smooth: Vec3,
    /// The step goal (cm).
    pub goal_cm: Vec3,
    /// A path is followed.
    pub path: bool,
    /// The climb/step count (`+0x860`).
    pub climb: i32,
    /// A foe's chest to look at (cm).
    pub foe_chest: Option<Vec3>,
    /// The held goal point passed the ground test.
    pub ground_clear: bool,
}

/// One tick of the step/combat walk driver.
///
/// With no path it does not walk. Otherwise the walk is aimed at the body's
/// goal: the look eases 0.1 toward the goal's direction (the facing itself
/// within 1 cm of it); the threshold is 0.7 past 100 cm, else 0.007 a cm.
/// Facing the goal within the threshold walks it and, past half the
/// threshold, steers; otherwise the goal is held within 150 cm of the body,
/// the walk is flagged and, when the held point is clear and the facing
/// within 0.9, it runs (sprinting at climb 2); the look yaw is the body's
/// look, its smoothing eases 0.1 toward it, and the head looks at the foe.
/// Returns the walk, or none while the path runs out.
pub fn walk(gaze: &mut Gaze, at: Walking) -> Option<Walk> {
    if !at.path {
        return None;
    }
    let delta = at.goal_cm - at.body_cm;
    let flat = Vec3::new(delta.x, 0.0, delta.z);
    let len2 = flat.length_squared();
    let (direction, dot) = if len2 <= 1.0 {
        (at.facing, 0.0)
    } else {
        let len = len2.sqrt();
        (flat / len, flat.dot(at.facing) / len)
    };
    let look = unit(at.look + (direction - at.look) * 0.1);
    let len = len2.sqrt();
    let threshold = if len > 100.0 { FAR_GOAL } else { len * 0.007 };
    let mut goal_cm = at.goal_cm;
    let (flags, steering) = if dot <= threshold {
        (MOVE, threshold * 0.5 < dot)
    } else {
        goal_cm = at.body_cm + flat * (GOAL_HOLD_CM / len);
        let mut flags = MOVE;
        if at.ground_clear && f64::from(dot) > 0.9 {
            flags |= RUN;
            if at.climb == 2 {
                flags |= SPRINT;
            }
        }
        (flags, true)
    };
    let look_yaw = yaw_of(look);
    let smooth = unit(at.look_smooth + (look - at.look_smooth) * 0.1);
    gaze.look = look_yaw;
    if let Some(chest) = at.foe_chest
        && let Some(head) = look_at(gaze, at.body_cm, chest)
    {
        return Some(Walk {
            goal_cm,
            look: head,
            flags,
            steering,
            look_smooth: smooth,
        });
    }
    gaze.offset =
        (f64::from(gaze.target_offset - gaze.offset) * 0.02 + f64::from(gaze.offset)) as f32;
    let look = dir_of(gap(gaze.look + gaze.offset, 0.0));
    Some(Walk {
        goal_cm,
        look,
        flags,
        steering,
        look_smooth: smooth,
    })
}

/// Stops the walk (the combatant's stop): the path and its replans clear,
/// the walk flags lose the run bit and gain the stopping bit, and a goal
/// farther than 8·crouch + 20 cm from the body is pulled to that distance
/// along the body's facing.
pub fn stop_walk(body_cm: Vec3, facing: Vec3, crouch: f32, mut walk: Walk) -> Walk {
    walk.flags = (walk.flags & !RUN) | STOPPING;
    walk.steering = false;
    let limit = crouch * STOP_CROUCH + STOP_BASE_CM;
    let d = walk.goal_cm - body_cm;
    if d.x * d.x + d.y * d.y + d.z * d.z > limit * limit {
        walk.goal_cm = body_cm + facing * limit;
    }
    walk
}

/// The AI's heading, look and body turn state out of combat: the heading
/// (`+0x8e0`), the look yaw (`+0x54`), the look offset (`+0x8d4`), the sway
/// (`+0x8dc`), and the clock the next draw is due (`+0x8ac`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Turning {
    pub heading: f32,
    pub look: f32,
    pub offset: f32,
    pub sway: f32,
    pub sway_at_ms: u32,
}

/// One tick of the out-of-combat body turn: the heading takes the look yaw,
/// the offset starts at the body's physical facing yaw; a draw at the clock
/// sets the sway to `unit·3.6 − 1.8` with the next draw 4000 +
/// high(roll·4000) ms out; the offset eases by `0.09 − 0.06·encumbrance`
/// toward the look plus the sway, wrapped once.
pub fn turn_towards(
    turn: &mut Turning,
    facing_yaw: f32,
    encumbrance: f32,
    now_ms: u32,
    roll: &mut u32,
) {
    turn.heading = turn.look;
    turn.offset = facing_yaw;
    if turn.sway_at_ms < now_ms {
        *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
        let unit = (*roll as f32) * (1.0 / 4_294_967_296.0);
        turn.sway = unit * SWAY_SPAN - 1.8;
        *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
        let step = ((u64::from(*roll) * 4000) >> 32) as u32;
        turn.sway_at_ms = now_ms.wrapping_add(SWAY_EVERY_MS).wrapping_add(step);
    }
    let target = gap(turn.look + turn.sway, 0.0);
    let rate = (0.09 - encumbrance * 0.06) as f64;
    turn.offset = (rate * f64::from(target - turn.offset) + f64::from(turn.offset)) as f32;
}

/// The state [`alert_look`] keeps: the look offset, its target and pitch,
/// and the two pan timers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlertLook {
    pub offset: f32,
    pub target: f32,
    pub pitch: f32,
    pub wide_at_ms: u32,
    pub narrow_at_ms: u32,
}

/// One tick of the alert look-around: an AI whose panic is under 0.2, or
/// that is in combat, does nothing. Past 1 panic the in-combat mark sets.
/// The body stops, its motion becomes the stand, and with no foe one draw
/// in 300 pans ±π about the body every 3 s (the next within 1 s), one draw
/// in 75 pans ±π/2 every 700 ms; the head eases 0.05 toward the pan and the
/// body's look smoothing follows it. With a foe the look turns toward it
/// (over a quarter turn the look takes the foe's yaw) and the body turns
/// ([`turn_towards`]). Returns the yaw the look smoothing should aim at.
#[allow(clippy::too_many_arguments, clippy::neg_cmp_op_on_partial_ord)]
pub fn alert_look(
    panic: f32,
    in_combat: bool,
    alert: &mut AlertLook,
    body_yaw: f32,
    foe_yaw: Option<f32>,
    now_ms: u32,
    roll: &mut u32,
) -> Option<f32> {
    if !(PANIC_FLOOR <= panic) || !in_combat {
        return None;
    }
    let Some(foe) = foe_yaw else {
        if alert.wide_at_ms < now_ms {
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            if ((u64::from(*roll) * WIDE_ODDS) >> 32) == 0 {
                *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
                let unit = (*roll as f32) * (1.0 / 4_294_967_296.0);
                alert.target = gap(body_yaw + (unit - 0.5) * TWO_PI, 0.0);
                alert.wide_at_ms = now_ms.wrapping_add(WIDE_EVERY_MS);
                alert.narrow_at_ms = now_ms.wrapping_add(1000);
            }
        }
        if alert.narrow_at_ms < now_ms {
            *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
            if ((u64::from(*roll) * NARROW_ODDS) >> 32) == 0 {
                *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
                let unit = (*roll as f32) * (1.0 / 4_294_967_296.0);
                alert.target = gap(body_yaw + (unit - 0.5) * std::f32::consts::PI, 0.0);
                alert.narrow_at_ms = now_ms.wrapping_add(NARROW_EVERY_MS);
            }
        }
        alert.offset =
            (f64::from(alert.target - alert.offset) * 0.05 + f64::from(alert.offset)) as f32;
        alert.pitch = 0.0;
        return Some(look_smoothing(body_yaw, alert.offset));
    };
    Some(foe)
}

/// The aim snap: the body's look yaw is taken as the AI's look, offset and
/// offset target.
pub fn snap_to_body(gaze: &mut Gaze, turn: &mut Turning, heading_yaw: f32) -> Vec3 {
    gaze.look = heading_yaw;
    gaze.offset = heading_yaw;
    gaze.target_offset = heading_yaw;
    gaze.sway = 0.0;
    turn.heading = heading_yaw;
    turn.look = heading_yaw;
    turn.offset = heading_yaw;
    dir_of(heading_yaw)
}

/// The aim: the look is snapped to the body's facing and a new sway draw is
/// made 1–4 s out.
pub fn aim(
    gaze: &mut Gaze,
    turn: &mut Turning,
    heading_yaw: f32,
    now_ms: u32,
    roll: &mut u32,
) -> Vec3 {
    let look = snap_to_body(gaze, turn, heading_yaw);
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    let step = ((u64::from(*roll) * 3000) >> 32) as u32;
    turn.sway_at_ms = now_ms.wrapping_add(1000).wrapping_add(step);
    look
}

/// The settle after leaving combat: the look takes the body's look yaw and
/// a new sway draw is made 1–4 s out.
pub fn settle_look(
    gaze: &mut Gaze,
    turn: &mut Turning,
    body_look_yaw: f32,
    now_ms: u32,
    roll: &mut u32,
) {
    gaze.look = body_look_yaw;
    gaze.offset = body_look_yaw;
    gaze.target_offset = body_look_yaw;
    turn.look = body_look_yaw;
    turn.heading = body_look_yaw;
    turn.offset = body_look_yaw;
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    let step = ((u64::from(*roll) * 3000) >> 32) as u32;
    turn.sway_at_ms = now_ms.wrapping_add(1000).wrapping_add(step);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(id: usize, x: f32, z: f32, dead: bool) -> BodyAt {
        BodyAt {
            id,
            position_cm: Vec3::new(x, 0.0, z),
            dead,
        }
    }

    /// The yaw of the four ground directions, the direction built back, and
    /// the engine's zero-safe unit.
    #[test]
    fn yaw_and_direction_agree() {
        assert!(yaw_of(Vec3::new(0.0, 0.0, 5.0)).abs() < 1e-6);
        assert!((yaw_of(Vec3::new(5.0, 0.0, 0.0)) - HALF_PI).abs() < 1e-6);
        assert!((yaw_of(Vec3::new(-5.0, 0.0, 0.0)) + HALF_PI).abs() < 1e-6);
        assert!((yaw_of(Vec3::new(0.0, 0.0, -5.0)).abs() - std::f32::consts::PI).abs() < 1e-5);
        assert_eq!(yaw_of(Vec3::ZERO), 0.0);
        assert!((dir_of(HALF_PI) - Vec3::X).length() < 1e-6);
        assert!((dir_of(0.0) - Vec3::Z).length() < 1e-6);
        assert!(
            (pitched(0.5, 1.0)
                - Vec3::new(
                    0.5f32.cos() * 1.0f32.sin(),
                    -0.5f32.sin(),
                    0.5f32.cos() * 1.0f32.cos()
                ))
            .length()
                < 1e-6
        );
        assert_eq!(unit(Vec3::ZERO), Vec3::ZERO);
        assert!((unit(Vec3::new(3.0, 4.0, 0.0)) - Vec3::new(0.6, 0.8, 0.0)).length() < 1e-6);
    }

    /// The gap wraps once and keeps a NaN.
    #[test]
    fn the_gap_wraps_once() {
        assert!((gap(3.5, 0.0) - (3.5 - TWO_PI)).abs() < 1e-5);
        assert!((gap(-3.5, 0.0) - (TWO_PI - 3.5)).abs() < 1e-5);
        assert!((gap(0.5, 0.8) + 0.3).abs() < 1e-6);
        assert!(gap(f32::NAN, 0.0).is_nan());
        assert_eq!(gap(0.5, 0.0), 0.5);
    }

    /// The head look eases its offset 2 % toward the point within a quarter
    /// turn and adds it to the look; a point behind writes nothing.
    #[test]
    fn the_head_look_eases_and_rejects_the_back() {
        let mut gaze = Gaze::at(0.0);
        let look = look_at(&mut gaze, Vec3::ZERO, Vec3::new(100.0, 0.0, 100.0)).unwrap();
        let target = std::f32::consts::FRAC_PI_4;
        assert!((gaze.target_offset - target).abs() < 1e-6);
        assert!((gaze.offset - (target * 0.02)).abs() < 1e-6);
        assert!((look - dir_of(target * 0.02)).length() < 1e-6);
        let mut gaze = Gaze::at(0.0);
        assert!(look_at(&mut gaze, Vec3::ZERO, Vec3::new(0.0, 0.0, -100.0)).is_none());
        assert!((gaze.target_offset.abs() - std::f32::consts::PI).abs() < 1e-5);
        let mut gaze = Gaze::at(0.0);
        assert!(look_at(&mut gaze, Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0)).is_some());
    }

    /// The look smoothing holds the gap and moves the look by the sway; the
    /// body's look is written only past 30°.
    #[test]
    fn the_look_smoothing_follows_the_heading() {
        let mut gaze = Gaze::at(0.0);
        assert!(smooth_look(&mut gaze, 1.0, 1.0, 30.0).is_some());
        assert!((gaze.sway - SWAY_RATE * 0.7).abs() < 1e-7);
        assert!((gaze.look - gaze.sway).abs() < 1e-7);
        let mut gaze = Gaze::at(0.0);
        gaze.look = 0.1;
        assert!(smooth_look(&mut gaze, 0.0, 0.0, 30.0).is_none());
        // A heading behind holds the gap at a quarter turn.
        let mut gaze = Gaze::at(0.0);
        smooth_look(&mut gaze, std::f32::consts::PI, 0.0, 0.0);
        assert!((gaze.sway + HALF_PI * SWAY_RATE * 0.7).abs() < 1e-6);
        // A look at exactly 30° is not rewritten.
        let mut gaze = Gaze::at(0.0);
        gaze.look = 30.0 * DEGREE;
        let body = gaze.look;
        assert!(smooth_look(&mut gaze, 0.0, body, 30.0).is_none());
    }

    /// The idle look turns the body's look 0.1 of the gap toward the target
    /// yaw and smooths the pitch 1 %; a running look target writes nothing.
    #[test]
    fn the_idle_look_turns_and_smooths_the_pitch() {
        let gaze = Gaze {
            offset: 1.0,
            pitch: 0.5,
            ..Gaze::default()
        };
        let at = Looking { look: Vec3::Z };
        let (look, yaw) = idle_look(&gaze, at, false);
        assert!((yaw - 0.1).abs() < 1e-6);
        let smooth = (0.5f64 * 0.01) as f32;
        assert!((look.unwrap() - pitched(smooth, 0.1)).length() < 1e-6);
        assert!(idle_look(&gaze, at, true).0.is_none());
        // The smoothing follows the facing by 0.4 of the gap, 0.64 beyond
        // 1.6 rad.
        assert!((look_smoothing(0.0, 1.0) - 0.4).abs() < 1e-6);
        assert!((look_smoothing(0.0, 2.0) - 0.64).abs() < 1e-6);
        assert!((look_smoothing(2.0, -1.0) - 1.36).abs() < 1e-6);
    }

    /// The walk holds a far goal within 150 cm, walks toward a near one, and
    /// runs only on clear ground with the facing within 0.9.
    #[test]
    fn the_walk_aims_holds_and_runs() {
        let base = Walking {
            body_cm: Vec3::ZERO,
            facing: Vec3::Z,
            look: Vec3::Z,
            look_smooth: Vec3::Z,
            goal_cm: Vec3::new(0.0, 0.0, 400.0),
            path: true,
            climb: 0,
            foe_chest: None,
            ground_clear: false,
        };
        let mut gaze = Gaze::at(0.0);
        let w = walk(&mut gaze, base).unwrap();
        assert_eq!(w.flags, MOVE);
        assert!(w.steering);
        assert!((w.goal_cm - Vec3::new(0.0, 0.0, 150.0)).length() < 1e-6);
        let mut gaze = Gaze::at(0.0);
        let w = walk(
            &mut gaze,
            Walking {
                ground_clear: true,
                ..base
            },
        )
        .unwrap();
        assert_eq!(w.flags, MOVE | RUN);
        let mut gaze = Gaze::at(0.0);
        let w = walk(
            &mut gaze,
            Walking {
                ground_clear: true,
                climb: 2,
                ..base
            },
        )
        .unwrap();
        assert_eq!(w.flags, MOVE | RUN | SPRINT);
        // No path: no walk.
        let mut gaze = Gaze::at(0.0);
        assert!(
            walk(
                &mut gaze,
                Walking {
                    path: false,
                    ..base
                }
            )
            .is_none()
        );
        // A goal 50 cm ahead is still held at 150 cm along its direction.
        let mut gaze = Gaze::at(0.0);
        let w = walk(
            &mut gaze,
            Walking {
                goal_cm: Vec3::new(50.0, 0.0, 50.0),
                ..base
            },
        )
        .unwrap();
        let along = Vec3::new(50.0, 0.0, 50.0).normalize() * 150.0;
        assert!((w.goal_cm - along).length() < 1e-4);
        assert!(
            (yaw_of(w.look) - 0.07272).abs() < 1e-4,
            "{}",
            yaw_of(w.look)
        );
        // A goal exactly 100 cm ahead is held at 150 cm too.
        let mut gaze = Gaze::at(0.0);
        let w = walk(
            &mut gaze,
            Walking {
                goal_cm: Vec3::new(0.0, 0.0, 100.0),
                ..base
            },
        )
        .unwrap();
        assert_eq!(w.flags, MOVE);
        assert!(w.steering);
        assert!((w.goal_cm - Vec3::new(0.0, 0.0, 150.0)).length() < 1e-5);
    }

    /// The stop clears the run bit, sets the stopping bit, and pulls a goal
    /// out to 8·crouch + 20 cm along the facing.
    #[test]
    fn the_stop_pulls_the_goal_in() {
        let walk = Walk {
            goal_cm: Vec3::new(0.0, 0.0, 100.0),
            flags: MOVE | RUN,
            steering: true,
            ..Walk::default()
        };
        let stopped = stop_walk(Vec3::ZERO, Vec3::Z, 0.5, walk);
        assert_eq!(stopped.flags, MOVE | STOPPING);
        assert!(!stopped.steering);
        assert!((stopped.goal_cm - Vec3::new(0.0, 0.0, 24.0)).length() < 1e-6);
        let walk = Walk {
            goal_cm: Vec3::new(0.0, 0.0, 10.0),
            ..Walk::default()
        };
        assert_eq!(
            stop_walk(Vec3::ZERO, Vec3::Z, 0.0, walk).goal_cm,
            Vec3::new(0.0, 0.0, 10.0)
        );
    }

    /// The steer line walks 120 cm at most and needs the end within 60 cm;
    /// a projection of zero fails.
    #[test]
    fn the_steer_line_reaches_sixty_centimetres() {
        assert!(steer_line(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 119.0)));
        assert!(steer_line(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 179.0)));
        assert!(!steer_line(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 181.0)));
        assert!(!steer_line(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, -10.0)));
        // A projection over one clamps: 500 cm out tests the 120 cm point.
        assert!(!steer_line(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, 500.0)));
        // A sideways point projects 0 and fails.
        assert!(!steer_line(Vec3::ZERO, Vec3::X, Vec3::new(0.0, 0.0, 50.0)));
    }

    /// The way share is the largest front-facing dot over the square, dead
    /// bodies and ones behind ignored.
    #[test]
    fn the_way_share_is_the_closest_in_front() {
        assert_eq!(way_share(Vec3::ZERO, Vec3::Z, &[], 150.0), 1.0);
        assert!(
            (way_share(Vec3::ZERO, Vec3::Z, &[body(1, 0.0, 50.0, false)], 150.0) - 0.98).abs()
                < 1e-6
        );
        assert!(
            (way_share(
                Vec3::ZERO,
                Vec3::Z,
                &[body(1, 0.0, -10.0, false), body(2, 0.0, 100.0, false)],
                150.0
            ) - 0.99)
                .abs()
                < 1e-6
        );
        assert_eq!(
            way_share(Vec3::ZERO, Vec3::Z, &[body(1, 0.0, 50.0, true)], 150.0),
            1.0
        );
    }

    /// The body in the way is the first live body the walk can be steered to
    /// once its point is moved 20 cm along the look.
    #[test]
    fn the_body_in_the_way_is_steered_to() {
        let bodies = [body(7, 0.0, 100.0, false), body(9, 0.0, 300.0, false)];
        assert_eq!(body_in_way(Vec3::ZERO, Vec3::Z, Vec3::Z, &bodies), Some(7));
        assert_eq!(
            body_in_way(Vec3::ZERO, Vec3::Z, Vec3::Z, &[body(1, 0.0, 100.0, true)]),
            None
        );
        // The 20 cm look shift puts a body 330 cm out beyond the clamp.
        assert_eq!(
            body_in_way(Vec3::ZERO, Vec3::Z, Vec3::Z, &[body(3, 0.0, 170.0, false)]),
            None
        );
    }

    /// The out-of-combat turn draws an offset of ±1.8 every four seconds
    /// and eases toward the look plus it.
    #[test]
    fn the_turn_sways_the_look() {
        let mut turn = Turning::default();
        let mut roll = 0u32;
        turn_towards(&mut turn, 0.5, 0.0, 1000, &mut roll);
        assert_eq!(turn.heading, 0.0);
        // The first draw: roll 1 → unit 2^-32, sway ≈ -1.8; next at 5000.
        assert!(turn.sway < -1.79);
        assert_eq!(turn.sway_at_ms, 5125);
        // The offset eases from the facing 0.5 toward the look (0) plus
        // the sway (-1.8) by 0.09 in the same tick.
        assert!((turn.offset - (0.5 + (-1.8 - 0.5) * 0.09)).abs() < 1e-4);
        turn_towards(&mut turn, 0.0, 0.0, 1000, &mut roll);
        assert!((turn.offset - (-1.8 * 0.09)).abs() < 1e-4);
        // At 0.5 encumbrance the rate is 0.06: the offset eases from the
        // facing 0 toward look 1 + sway 1.
        let mut turn = Turning {
            look: 1.0,
            sway: 1.0,
            sway_at_ms: 10_000,
            ..Turning::default()
        };
        turn_towards(&mut turn, 0.0, 0.5, 0, &mut roll);
        assert!((turn.offset - 0.12).abs() < 1e-6);
    }

    /// The alert look does nothing under 0.2 panic and pans only on its
    /// draws; a foe turns the look to the foe's yaw.
    #[test]
    fn the_alert_look_pans_on_its_draws() {
        let mut alert = AlertLook::default();
        let mut roll = 0u32;
        assert!(alert_look(0.1, true, &mut alert, 0.0, None, 0, &mut roll).is_none());
        let mut alert = AlertLook::default();
        let mut roll = 0u32;
        // Roll 1: 1·300 >> 32 = 0, so the wide pan draws on this tick.
        let yaw = alert_look(0.5, true, &mut alert, 0.0, None, 10, &mut roll).unwrap();
        assert_eq!(alert.wide_at_ms, 3010);
        assert_eq!(alert.narrow_at_ms, 1010);
        // The draw after the first: roll 134775814 → unit 0.031377, so the
        // target is (0.031377 - 0.5)·2π = -2.94443.
        assert!((alert.target + 2.94443).abs() < 1e-4, "{}", alert.target);
        assert!((alert.offset - alert.target * 0.05).abs() < 1e-3);
        assert!(yaw.is_finite());
        // A foe: the yaw points at it.
        let mut alert = AlertLook::default();
        let mut roll = 0u32;
        let yaw = alert_look(0.5, true, &mut alert, 0.0, Some(HALF_PI), 10, &mut roll).unwrap();
        assert!((yaw - HALF_PI).abs() < 1e-6);
    }

    /// The aim and settle take the body's yaw and draw a 1–4 s sway timer;
    /// the snap takes the heading into all three AI yaws.
    #[test]
    fn the_aim_settle_and_snap_take_the_body_yaw() {
        let mut gaze = Gaze::default();
        let mut turn = Turning::default();
        let mut roll = 0u32;
        assert!((aim(&mut gaze, &mut turn, 0.5, 100, &mut roll) - dir_of(0.5)).length() < 1e-6);
        assert_eq!((gaze.look, gaze.offset, turn.heading), (0.5, 0.5, 0.5));
        assert_eq!(turn.sway_at_ms, 1100);
        let mut gaze = Gaze::default();
        let mut turn = Turning::default();
        settle_look(&mut gaze, &mut turn, 0.25, 200, &mut roll);
        assert_eq!((gaze.look, turn.heading, turn.offset), (0.25, 0.25, 0.25));
        assert_eq!(turn.sway_at_ms, 1294);
        let mut gaze = Gaze::default();
        let mut turn = Turning::default();
        snap_to_body(&mut gaze, &mut turn, -0.5);
        assert_eq!(
            (gaze.look, gaze.offset, gaze.target_offset, turn.look),
            (-0.5, -0.5, -0.5, -0.5)
        );
    }
}
