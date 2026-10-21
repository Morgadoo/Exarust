//! The player behaviour's own rules, outside its steering: the aim point and
//! its soft height lock, the near-contact list, the combat target picked
//! nearest the cursor, the heading followers, the attack timing and the
//! stance it asks for. Rules: `docs/PLAYER.md`.
//!
//! These are the parts of the original's player behaviour that are not the
//! locomotion request ([`crate::steering`]) nor the combat swing
//! (`exa_combat`). They are plain functions so that the arithmetic, the
//! comparisons and the branch order are pinned by tests; the running game
//! feeds them the cursor, the camera and the fighters it can see.
//!
//! Evidence: the behaviour's aim point, contacts, auto-target, heading
//! smoothers, attack timing and stance helper, read in the instructions
//! (the `aim_point`, `heading_follow`, `attack_timing` and `stance_choice`
//! tests below carry the numbers).

use bevy::math::{Vec2, Vec3};

/// The engine's single-precision epsilon, the floor under its "is it zero"
/// energy tests.
pub const EPSILON: f32 = 1.192_089_9e-7;
/// A character is in the player's contact list within 12 m (cm).
pub const CONTACT_RANGE_CM: f32 = 1200.0;
/// The contact list holds this many characters.
pub const CONTACTS: usize = 12;
/// The combat target is picked within 5 m (cm).
pub const TARGET_RANGE_CM: f32 = 500.0;
/// The stance helper takes the target's own scale times this (cm): nearer
/// than it, the combat idle; beyond it, the guard.
pub const TARGET_NEAR_CM: f32 = 350.0;
/// An attack keeps the combat idle for 3000 ms of the game clock after its
/// last tick.
pub const ATTACK_READY_MS: u64 = 3000;
/// The aim plane lies this far (cm) over the player's ground.
pub const AIM_PLANE_CM: f32 = 80.0;
/// A gamepad aim this far out (squared, compared in double precision) is
/// taken; its point is `250·|aim| + 50` cm along it.
pub const AIM_DEAD_ZONE: f64 = 0.01;
pub const AIM_REACH: f32 = 250.0;
pub const AIM_MARGIN: f32 = 50.0;
/// The cursor's screen ease per tick: `(0.4·AimSpeed + 0.1)²`.
pub const AIM_EASE: f64 = 0.4;
pub const AIM_EASE_MARGIN: f64 = 0.1;
/// An aim height lock: within twice the cursor radius the height blends,
/// fully within it.
pub const AIM_LOCK_RADII: f32 = 2.0;
/// Without a lock, a picked surface raises the aim point at most this far
/// (cm) above the player's ground.
pub const RAISED_SURFACE_MAX_CM: f32 = 200.0;
/// The heading followers' laws.
const HEAD_GAIN: f32 = 0.015_707_964;
const HEAD_KEEP: f32 = 1.0 / 3.0;
const HEAD_CLAMP: f32 = std::f32::consts::FRAC_PI_2;
const HEAD_STILL: f64 = 1e-5;
/// The body's step toward its aim, per tick (0.1π rad).
const BODY_STEP: f32 = std::f32::consts::PI * 0.1;
/// The strongest engaged enemy must lie within the player's reach (cm)
/// plus this, and within about 72.5° of its facing (the cosine's 0.3).
pub const ENGAGED_MARGIN_CM: f32 = 70.0;
pub const ENGAGED_COSINE: f32 = 0.3;
/// The minimum distance at which the contact and engaged tests accept a
/// character (cm, squared): the player itself is skipped.
const SELF_RANGE2: f32 = 1.0;
/// The wind-up step's starting value and floors (the original's arc of the
/// swing's strength).
pub const WINDUP_START: f32 = 0.025;
pub const WINDUP_FLOOR: f32 = 0.1;
pub const WINDUP_LEAST: f32 = 0.01;
/// The wind-up and recovery rates.
const WINDUP_PER_FRAME: f64 = 0.03;
const WINDUP_FRAMES: f32 = 80.0;
const WINDUP_MS: f32 = 1000.0;
const RECOVERY_BALANCE: f64 = 0.7;
const RECOVERY_SLOPE: f64 = 1.428_571_5;

/// Wraps `target` (a yaw) to within half a turn of `current`.
pub fn wrap_near(current: f32, target: f32) -> f32 {
    if current - target > std::f32::consts::PI {
        target + std::f32::consts::TAU
    } else if target - current > std::f32::consts::PI {
        target - std::f32::consts::TAU
    } else {
        target
    }
}

/// A yaw reduced into `[0, 2π)` the way the follower does it: the floor
/// division's remainder, then one turn either way.
pub fn wrap_positive(yaw: f32) -> f32 {
    let mut y = yaw - (yaw / std::f32::consts::TAU).trunc() * std::f32::consts::TAU;
    if y < 0.0 {
        y += std::f32::consts::TAU;
    } else if y > std::f32::consts::TAU {
        y -= std::f32::consts::TAU;
    }
    y
}

/// The planar yaw of a vector, the engine's arc cosine of `z` over the
/// horizontal length, with the sign of `x`: the same angle as
/// [`exa_physics::body::locomotion::yaw_of`].
pub fn yaw_of(vector: Vec3) -> f32 {
    exa_physics::body::locomotion::yaw_of(vector)
}

/// The heading follower of the combat aim: a target vector and yaw, a turn
/// speed, and the body yaw it is published as.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AimFollower {
    /// The asked direction (its yaw is what the follower seeks).
    pub target: f32,
    /// The shown yaw (rad, in `[0, 2π)` after a turn) and its speed.
    pub yaw: f32,
    speed: f32,
}

impl Default for AimFollower {
    fn default() -> Self {
        Self {
            target: 0.0,
            yaw: 0.0,
            speed: 0.0,
        }
    }
}

impl AimFollower {
    /// A follower resting at `yaw`.
    pub fn at(yaw: f32) -> Self {
        let yaw = wrap_positive(yaw);
        Self {
            target: yaw,
            yaw,
            speed: 0.0,
        }
    }

    /// Takes a new target vector: its yaw, wrapped to within half a turn of
    /// the follower's, and the speed cleared.
    pub fn follow(&mut self, vector: Vec3) {
        self.target = wrap_near(self.yaw, yaw_of(vector));
        self.speed = 0.0;
    }

    /// Sets the target yaw directly (the original's set-yaw helper): the
    /// angle is reduced into a turn, then wrapped near the shown yaw.
    pub fn follow_yaw(&mut self, yaw: f32) {
        let reduced = yaw - (yaw / std::f32::consts::TAU).trunc() * std::f32::consts::TAU;
        self.target = wrap_near(self.yaw, reduced);
        self.speed = 0.0;
    }

    /// One tick: the error is clamped to ±π/2, `speed = (speed +
    /// 0.015708·error) / 3`, and the yaw moves by the speed when it is over
    /// 1e-5 (compared in double precision), then wraps into a turn.
    pub fn tick(&mut self) {
        let error = (wrap_near(self.yaw, self.target) - self.yaw).clamp(-HEAD_CLAMP, HEAD_CLAMP);
        self.speed = (self.speed + error * HEAD_GAIN) * HEAD_KEEP;
        if f64::from(self.speed.abs()) > HEAD_STILL {
            self.yaw = wrap_positive(self.yaw + self.speed);
        }
    }
}

/// Turns a body yaw toward `aim` by at most 0.1π rad: the original's
/// per-tick step toward a look point. Returns the new yaw (wrapped near the
/// old one).
pub fn turn_step(yaw: f32, aim: f32) -> f32 {
    let diff = wrap_near(yaw, aim) - yaw;
    if diff.abs() > BODY_STEP {
        let step = if diff < 0.0 { -BODY_STEP } else { BODY_STEP };
        wrap_near(yaw, yaw + step)
    } else {
        yield_near(yaw, aim)
    }
}

/// `aim`, wrapped into the halfturn around `yaw` (the original's wrap of
/// the difference).
fn yield_near(yaw: f32, aim: f32) -> f32 {
    let mut a = aim;
    if yaw - a > std::f32::consts::PI {
        a += std::f32::consts::TAU;
    } else if a - yaw > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    }
    a
}

/// Whether a gamepad aim is taken: its squared length over 0.01, compared
/// in double precision.
pub fn aim_engaged(aim: Vec2) -> bool {
    let x = f64::from(aim.x);
    let z = f64::from(aim.y);
    x * x + z * z > AIM_DEAD_ZONE
}

/// The aim point's distance (cm) for a stick of magnitude `m`: 250·m + 50.
pub fn aim_distance(m: f32) -> f32 {
    m * AIM_REACH + AIM_MARGIN
}

/// The aim cursor's ease factor: `(0.4·AimSpeed + 0.1)²`, worked in double
/// precision, then single.
pub fn aim_factor(aim_speed: f32) -> f32 {
    (f64::from(aim_speed) * AIM_EASE + AIM_EASE_MARGIN).powi(2) as f32
}

/// The screen cursor after one tick of the gamepad ease: the point moves
/// [`aim_factor`] of the way toward the asked screen point.
pub fn aim_cursor_step(current: Vec2, want: Vec2, aim_speed: f32) -> Vec2 {
    current + (want - current) * aim_factor(aim_speed)
}

/// The aim point in the world after one tick of the gamepad ease.
pub fn aim_point_step(current: Vec3, want: Vec3, aim_speed: f32) -> Vec3 {
    current + (want - current) * aim_factor(aim_speed)
}

/// The aim point's height when an opponent's bone shows within reach of the
/// cursor: `bone` is the bone's height, `distance` its screen distance and
/// `radius` the pixel radius. Within the radius the height is the bone's;
/// within twice it, the two blend by `(d − r) / r`; beyond, the plane's.
pub fn aim_lock_height(plane: f32, bone: f32, distance: f32, radius: f32) -> f32 {
    if distance < radius * AIM_LOCK_RADII * radius * AIM_LOCK_RADII {
        if distance <= radius * radius {
            bone
        } else {
            let blend = (distance.sqrt() - radius) / radius;
            plane * blend + (1.0 - blend) * bone
        }
    } else {
        plane
    }
}

/// The aim point, raised onto a surface the cursor's pick met: the surface's
/// height `hit` when it is above the plane and below the player's ground
/// plus 200 cm, else the plane.
pub fn aim_raised(plane: f32, hit: f32, ground: f32) -> f32 {
    if plane < hit && hit < ground + RAISED_SURFACE_MAX_CM {
        hit
    } else {
        plane
    }
}

/// A character near the player, as the player's contact list keeps it: the
/// body's id, the closeness (0–0.99), the look and facing weight, and their
/// combined score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub body: u64,
    pub score: f32,
    pub closeness: f32,
    pub facing: f32,
}

/// Builds the player's contact list, the original's NPC-perception-like
/// pass over every character within 12 m: the closeness falls from 1 by
/// `d · 0.00083333335` (d in cm) and stops at 0.99; the facing weight is
/// `0.75 · (0.5 − 0.5·facing·dir) + 0.25 · (0.5 − 0.5·look·dir)²`; the
/// score is half of each. The list holds twelve, lowest score evicted.
pub fn contacts(
    player: Vec3,
    facing: Vec3,
    look: Vec3,
    others: impl IntoIterator<Item = (u64, Vec3)>,
) -> Vec<Contact> {
    let mut out: Vec<Contact> = Vec::with_capacity(CONTACTS);
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
    let facing = flat(facing).normalize_or_zero();
    let look = flat(look).normalize_or_zero();
    for (body, at) in others {
        let delta = flat(at - player);
        let d2 = delta.z * delta.z + delta.x * delta.x;
        if d2 >= CONTACT_RANGE_CM * CONTACT_RANGE_CM {
            continue;
        }
        let d = d2.sqrt();
        let dir = if d > 0.0 { delta / d } else { Vec3::ZERO };
        let mut closeness = 1.0 - d * 0.000_833_333_35;
        if closeness > 0.99 {
            closeness = 0.99;
        }
        let facing_part =
            (facing.dot(dir) * -0.5 + 0.5).powi(2) * 0.25 + (look.dot(dir) * -0.5 + 0.5) * 0.75;
        let score = facing_part * 0.5 + closeness * 0.5;
        let entry = Contact {
            body,
            score,
            closeness,
            facing: facing_part,
        };
        if let Some(slot) = out.iter_mut().find(|c| c.body == body) {
            *slot = entry;
        } else if out.len() < CONTACTS {
            out.push(entry);
        } else {
            // The lowest scoring contact gives way to a better one (the
            // original starts at the first and takes strict improvements).
            let mut lowest = 0;
            for i in 1..out.len() {
                if out[i].score < out[lowest].score {
                    lowest = i;
                }
            }
            if out[lowest].score < score {
                out[lowest] = entry;
            }
        }
    }
    out
}

/// Whether a contact is engaged: its score over 2.0 (the original's
/// comparison; the contact scores of this pass are under that, so a
/// character is never engaged through them — a quirk kept).
pub fn engaged(contact: &Contact) -> bool {
    contact.score > 2.0
}

/// A character the player could reach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearFighter {
    pub body: u64,
    pub at: Vec3,
    /// The character's scale (its body's size factor).
    pub scale: f32,
    /// The threat the strongest-engaged test weighs.
    pub threat: f32,
}

/// The strongest engaged enemy in front within the player's reach plus
/// 70 cm: the dot with the player's look is at least zero and over 0.3 of
/// the distance (within about 72.5°), the distance squared is at least 1
/// and within the reach, and the contact is engaged. Returns whether one
/// was found and the largest threat among them.
pub fn strongest_engaged(
    player: Vec3,
    look: Vec3,
    reach: f32,
    fighters: &[NearFighter],
    contact_list: &[Contact],
) -> (bool, f32) {
    let mut found = false;
    let mut best = 0.0f32;
    let limit = reach + ENGAGED_MARGIN_CM;
    for f in fighters {
        let delta = f.at - player;
        let d2 = delta.z * delta.z + delta.x * delta.x + delta.y * delta.y;
        // The ordered tests refuse an over-long, too-near or behind
        // character; a NaN compares unordered and is refused, as the
        // original's conjunction is.
        if d2 > limit * limit || d2 < SELF_RANGE2 || d2.is_nan() {
            continue;
        }
        let dot = delta.dot(look);
        if dot < 0.0 || dot.is_nan() || d2.sqrt() * ENGAGED_COSINE >= dot {
            continue;
        }
        if !contact_list
            .iter()
            .find(|c| c.body == f.body)
            .is_some_and(engaged)
        {
            continue;
        }
        // The original's strict improvement: a NaN threat passes neither.
        if best <= f.threat && f.threat != best {
            best = f.threat;
        }
        found = true;
    }
    (found, best)
}

/// The combat target the player picks: among the other living fighters the
/// cursor can see within 5 m, the one that maximises `(500 − d)·(cos + 0.5)`
/// with the angle measured from the cursor's direction (so a target behind
/// it scores low, never a hard cutoff). Returns the body and its point.
pub fn auto_target(
    player: Vec3,
    cursor: Vec3,
    others: impl IntoIterator<Item = (u64, Vec3, bool, bool)>,
) -> Option<(u64, Vec3)> {
    let mut best: Option<(u64, Vec3)> = None;
    let mut best_score = 0.0f32;
    for (body, point, dead, visible) in others {
        // The original skips bit 0x800 (dead) and asks the viewport for
        // visibility.
        if dead || !visible {
            continue;
        }
        let delta = point - player;
        let d2 = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;
        if d2 > TARGET_RANGE_CM * TARGET_RANGE_CM {
            continue;
        }
        let d = d2.sqrt();
        let cos = if d > 0.0 { delta.dot(cursor) / d } else { 0.0 };
        let score = (TARGET_RANGE_CM - d) * (cos + 0.5);
        if best.is_none() || best_score < score {
            best = Some((body, point));
            best_score = score;
        }
    }
    best
}

/// The character closest to the cursor on screen: the squared screen
/// distance to each living fighter's head, the nearest wins. `screen_of`
/// gives a world point's window position.
pub fn closest_to_cursor(
    cursor: Vec2,
    others: impl IntoIterator<Item = (u64, Vec2, bool)>,
) -> Option<u64> {
    let mut best: Option<(u64, f32)> = None;
    for (body, screen, dead) in others {
        if dead {
            continue;
        }
        let d2 = (screen.x - cursor.x).powi(2) + (screen.y - cursor.y).powi(2);
        if best.is_none_or(|(_, b)| d2 < b) {
            best = Some((body, d2));
        }
    }
    best.map(|(b, _)| b)
}

/// The stance helper's value: 4 (the combat idle) when it is alive (health
/// over 0.01), a target is within 350 cm times its scale or its last attack
/// tick was under 3 s ago, and it is not dazed (health within 0.05 of its
/// most); else 3 (the guard).
pub fn stance_choice(
    yellow: f32,
    dazed: bool,
    target: Option<(f32, f32)>,
    since_attack_ms: Option<u64>,
) -> u8 {
    if yellow >= 0.01
        && (target.is_some_and(|(distance, scale)| distance <= TARGET_NEAR_CM * scale)
            || since_attack_ms.is_some_and(|ms| ms < ATTACK_READY_MS))
        && !dazed
    {
        4
    } else {
        3
    }
}

/// The attack timing after a swing's tick: its wind-up step, the game clock
/// (ms) of the next attack and of the recovery. `energy` is the fighter's
/// energy over its most, `swing_frames` its swing's expected length, and
/// `rate` the weapon's rate (the original's W17). `balance` and `mode` (the
/// leg mode, 5 to 12 meaning a swing) with `no_recovery_flag` give the
/// recovery's quadratic.
pub fn windup(
    energy: f32,
    rate: f32,
    swing_frames: u32,
    balance: f64,
    mode: i32,
    no_recovery_flag: bool,
    now_ms: u32,
) -> Windup {
    let mut step = if energy > EPSILON {
        (1.0 - f64::from(energy.sqrt()) * 0.05) as f32
    } else {
        1.0
    };
    if f64::from(step) < 0.1 {
        step = 0.1;
    }
    step = (f64::from(step / rate) * WINDUP_PER_FRAME) as f32;
    if f64::from(step) < 0.01 {
        step = 0.01;
    }
    // The engine casts to an integer with the nearest-even mode.
    let by_step = (WINDUP_MS / (WINDUP_FRAMES * step)).round_ties_even() as u32;
    let frames = by_step.min(swing_frames);
    let next_attack = now_ms.wrapping_add(frames);
    let factor = if balance > RECOVERY_BALANCE || !(5..=12).contains(&mode) || no_recovery_flag {
        1.0
    } else {
        let f = (balance * RECOVERY_SLOPE) as f32;
        (3.0 - f * 2.0) * f * f
    };
    let weapon = (rate * 100.0).round_ties_even() as i32 + 100;
    let recovery = (factor * 2.0 + 1.0) * weapon as f32;
    Windup {
        step,
        next_attack,
        recovery: now_ms.wrapping_add(recovery.round_ties_even() as u32),
        recovery_factor: factor,
    }
}

/// The attack timing the behaviour keeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Windup {
    /// The swing strength's drain per tick.
    pub step: f32,
    /// The game clock (ms) of the next attack.
    pub next_attack: u32,
    /// The game clock (ms) the recovery ends.
    pub recovery: u32,
    /// The recovery's quadratic factor.
    pub recovery_factor: f32,
}

/// The combat aim follower and the screen cursor's ease are the two
/// per-tick aim pieces the controls and the behaviour share.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AimState {
    pub follower: AimFollower,
    pub cursor: Vec2,
}

impl AimState {
    /// One tick of a gamepad aim vector: the follower takes it when it is
    /// over the dead zone, and the screen cursor eases toward the asked
    /// point (passed in) by [`aim_cursor_step`].
    pub fn tick(&mut self, aim: Option<Vec3>, want: Vec2, aim_speed: f32) {
        if let Some(aim) = aim.filter(|a| aim_engaged(Vec2::new(a.x, a.z))) {
            self.follower.follow(aim);
        }
        self.follower.tick();
        self.cursor = aim_cursor_step(self.cursor, want, aim_speed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(d: f32) -> f32 {
        d.to_radians()
    }

    /// The aim cursor's ease is (0.4·AimSpeed + 0.1)² a tick: at the
    /// shipped 0.29 it is 0.216² = 0.046656.
    #[test]
    fn the_aim_cursor_eases_by_the_aim_speed_squared_sum() {
        let want = Vec2::new(100.0, 200.0);
        let got = aim_cursor_step(Vec2::ZERO, want, 0.29);
        assert!((got.x - 4.665_6).abs() < 1e-4, "{got}");
        assert!((got.y - 9.331_2).abs() < 1e-4, "{got}");
        // At AimSpeed 0 it is still a tenth squared.
        let slow = aim_cursor_step(Vec2::ZERO, want, 0.0);
        assert!((slow.x - 1.0).abs() < 1e-6);
        // A full ease arrives at once (the factor is over one).
        let fast = aim_cursor_step(Vec2::ZERO, want, 2.25);
        assert!((fast - want).length() < 1e-4);
        // The world point eases by the same factor.
        let p = aim_point_step(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 0.29);
        assert!((p.x - 4.6656).abs() < 1e-4, "{p}");
    }

    /// The aim point is 250·|aim| + 50 cm out; under 0.01 squared it is not
    /// taken, and the comparison is in double precision.
    #[test]
    fn the_gampad_aims_point_and_dead_zone() {
        assert_eq!(aim_distance(0.0), 50.0);
        assert_eq!(aim_distance(1.0), 300.0);
        assert!(!aim_engaged(Vec2::new(0.09, 0.0)));
        assert!(aim_engaged(Vec2::new(0.1, 0.0)));
        assert!(!aim_engaged(Vec2::new(0.05, 0.05)));
        assert!(aim_engaged(Vec2::new(0.08, 0.08)));
    }

    /// An aim point is fully locked within one cursor radius, blends by
    /// (d − r)/r to two radii, and keeps the plane beyond them.
    #[test]
    fn the_aim_locks_onto_a_bone_within_two_radii() {
        assert_eq!(aim_lock_height(0.8, 1.5, 25.0, 10.0), 1.5);
        // 15 px: blend (15 − 10)/10 = 0.5.
        assert!((aim_lock_height(0.8, 1.5, 225.0, 10.0) - 1.15).abs() < 1e-5);
        assert_eq!(aim_lock_height(0.8, 1.5, 400.0, 10.0), 0.8);
    }

    /// A raised surface under the cursor lifts the aim point above the
    /// plane, up to 200 cm over the player's ground.
    #[test]
    fn a_picked_surface_raises_the_aim_point() {
        // Plane 80 cm, a table top at 120 cm.
        assert_eq!(aim_raised(80.0, 120.0, 0.0), 120.0);
        // The plane is the floor of the lift, the ground plus 200 the cap.
        assert_eq!(aim_raised(80.0, 70.0, 0.0), 80.0);
        assert_eq!(aim_raised(80.0, 250.0, 0.0), 80.0);
        assert_eq!(aim_raised(80.0, 240.0, 50.0), 240.0);
        assert_eq!(aim_raised(80.0, 250.0, 50.0), 80.0);
    }

    /// The contacts hold twelve bodies; closeness falls 0.00083333335 a cm
    /// from 1, capped at 0.99; the score is half the facing weight and half
    /// the closeness.
    #[test]
    fn the_contact_list_weighs_closeness_and_facing() {
        let here = Vec3::ZERO;
        let forward = Vec3::Z;
        // 100 cm ahead: closeness 1 − 0.083333335 = 0.9166666; facing
        // weight 0.75·0 + 0.25·1 = 0.25 (the look part squared is 1? No:
        // (0.5 − 0.5·1)² = 0, so 0.25·0 = 0; both look and facing ahead
        // give zero).
        let c = contacts(here, forward, forward, [(1, Vec3::new(0.0, 0.0, 100.0))]);
        assert_eq!(c.len(), 1);
        let c = c[0];
        assert!((c.closeness - 0.916_666_7).abs() < 1e-6, "{c:?}");
        assert!(c.facing.abs() < 1e-6, "{c:?}");
        assert!((c.score - 0.458_333_35).abs() < 1e-6, "{c:?}");
        // Behind: both weights are 1 (0.75 + 0.25).
        let behind = contacts(here, forward, forward, [(1, Vec3::new(0.0, 0.0, -100.0))])[0];
        assert!((behind.facing - 1.0).abs() < 1e-6, "{behind:?}");
        assert!((behind.score - 0.958_333_35).abs() < 1e-6, "{behind:?}");
        // 12 m is the end of the list: it is not held.
        assert!(contacts(here, forward, forward, [(1, Vec3::Z * 1200.0)]).is_empty());
        assert_eq!(
            contacts(here, forward, forward, [(1, Vec3::Z * 1199.0)]).len(),
            1
        );
        // Thirteen bodies keep the twelve nearest-ahead: the farthest
        // behind is evicted.
        let many: Vec<_> = (0..13)
            .map(|i| (i as u64, Vec3::new(0.0, 0.0, 100.0 + i as f32)))
            .collect();
        let kept = contacts(here, forward, forward, many);
        assert_eq!(kept.len(), 12);
        assert!(kept.iter().all(|c| c.body != 12), "{kept:?}");
        // The contact scores can never pass the engaged test's 2.0.
        assert!(kept.iter().all(|c| !engaged(c)));
    }

    /// `(500 − d)·(cos + 0.5)`: a near opponent in front beats a far one;
    /// behind it scores low but still counts; the dead, invisible and
    /// beyond 500 cm are left out.
    #[test]
    fn the_combat_target_is_the_best_scored_opponent() {
        let cursor = Vec3::Z;
        let got = auto_target(
            Vec3::ZERO,
            cursor,
            [
                (1, Vec3::new(0.0, 0.0, 400.0), false, true),
                (2, Vec3::new(0.0, 0.0, 100.0), false, true),
                (3, Vec3::new(0.0, 0.0, 600.0), false, true),
                (4, Vec3::new(0.0, 0.0, 100.0), true, true),
                (5, Vec3::new(0.0, 0.0, 100.0), false, false),
            ],
        );
        assert_eq!(got.map(|t| t.0), Some(2));
        // 200 cm to the side at 100 cm: cos 0.447, score 400 × 0.947.
        let side = auto_target(
            Vec3::ZERO,
            cursor,
            [(1, Vec3::new(200.0, 0.0, 0.0), false, true)],
        );
        assert!(side.is_some());
        // Behind: cos −1, score 0 at any distance.
        let behind = auto_target(
            Vec3::ZERO,
            cursor,
            [(1, Vec3::new(0.0, 0.0, -100.0), false, true)],
        );
        assert!(behind.is_some_and(|(_, p)| p.z == -100.0));
        assert!(auto_target(Vec3::ZERO, cursor, []).is_none());
    }

    /// The character closest to the cursor's screen point, the dead left
    /// out.
    #[test]
    fn the_dialogue_target_is_he_who_is_closest_on_screen() {
        let got = closest_to_cursor(
            Vec2::new(100.0, 100.0),
            [
                (1, Vec2::new(150.0, 150.0), false),
                (2, Vec2::new(110.0, 110.0), false),
                (3, Vec2::new(101.0, 101.0), true),
            ],
        );
        assert_eq!(got, Some(2));
    }

    /// The heading follower's step: error over π/2 clamps; speed =
    /// (speed + 0.015708·e)/3; a speed of 1e-5 or less does not move it.
    #[test]
    fn the_aim_follower_turns_in_thirds() {
        let mut f = AimFollower::at(0.0);
        f.follow(Vec3::X); // a quarter turn to +π/2
        f.tick();
        let speed = (std::f32::consts::FRAC_PI_2 * HEAD_GAIN) / 3.0;
        assert!((f.speed - speed).abs() < 1e-8, "{f:?}");
        assert!((f.yaw - speed).abs() < 1e-8, "{f:?}");
        // The ⅓ gain settles slowly (its slow pole is about 0.992 a tick):
        // after 300 ticks (3.6 s) it is still 0.145 rad short, and the
        // 1e-5 speed floor stops it 1.3 mrad short for good.
        for _ in 0..300 {
            f.tick();
        }
        assert!((f.yaw - std::f32::consts::FRAC_PI_2).abs() < 0.2, "{f:?}");
        for _ in 300..3000 {
            f.tick();
        }
        assert!((f.yaw - std::f32::consts::FRAC_PI_2).abs() < 0.002, "{f:?}");
    }

    /// Yaws are wrapped to within half a turn of the current one.
    #[test]
    fn a_yaw_wraps_the_short_way() {
        assert!((wrap_near(0.1, 6.1) + 0.183_185).abs() < 1e-4);
        assert!((wrap_near(6.1, 0.1) - 6.383_185).abs() < 1e-4);
        assert_eq!(wrap_near(1.0, 1.5), 1.5);
        assert_eq!(wrap_positive(-0.5), std::f32::consts::TAU - 0.5);
        assert_eq!(wrap_positive(std::f32::consts::TAU + 0.5), 0.5);
    }

    /// The body steps at most 0.1π rad toward its aim a tick.
    #[test]
    fn the_body_steps_a_tenth_of_a_turn() {
        assert!((turn_step(0.0, std::f32::consts::PI) - BODY_STEP).abs() < 1e-6);
        assert!((turn_step(0.0, deg(10.0)) - deg(10.0)).abs() < 1e-6);
        assert!((turn_step(0.0, deg(30.0)) - deg(18.0)).abs() < 1e-6);
        assert!((turn_step(0.0, deg(-30.0)) + deg(18.0)).abs() < 1e-6);
    }

    /// The stance helper: 4 with a target nearer than 350 cm × its scale,
    /// or within 3 s of an attack, unless dazed; 3 otherwise.
    #[test]
    fn the_stance_helper_picks_the_combat_idle_or_the_guard() {
        assert_eq!(stance_choice(0.5, false, Some((349.0, 1.0)), None), 4);
        assert_eq!(stance_choice(0.5, false, Some((351.0, 1.0)), None), 3);
        // An ogre at scale 2 is near to 700 cm.
        assert_eq!(stance_choice(0.5, false, Some((690.0, 2.0)), None), 4);
        assert_eq!(stance_choice(0.5, false, None, Some(2999)), 4);
        assert_eq!(stance_choice(0.5, false, None, Some(3000)), 3);
        assert_eq!(stance_choice(0.5, true, Some((100.0, 1.0)), None), 3);
        assert_eq!(stance_choice(0.009, false, Some((100.0, 1.0)), None), 3);
    }

    /// The wind-up: from 0.025, scaled by 1 − 0.05·√E (1 when the energy is
    /// zero), over the weapon's rate, 0.03, floored at 0.01; the next attack
    /// is 1000/(80·step) ms on, at most the swing's own length.
    #[test]
    fn the_attack_timing_drains_from_the_wind_up() {
        let w = windup(1.0, 0.1, 1000, 1.0, 0, false, 10_000);
        // step = 0.95/0.1*0.03 = 0.285, so 1000/(80·0.285) = 43.86 → 44.
        assert!((w.step - 0.285).abs() < 1e-5, "{w:?}");
        assert_eq!(w.next_attack, 10_044);
        // The recovery factor 1: (100·0.1 + 100)·3 = 330 ms.
        assert_eq!(w.recovery, 10_330);
        // A swing only ten frames long caps the wait.
        assert_eq!(windup(1.0, 0.1, 10, 1.0, 0, false, 0).next_attack, 10);
        // Zero energy takes the 1.0 step: 1000/(80·0.3) = 41.67 → 42.
        assert_eq!(windup(0.0, 0.1, 1000, 1.0, 0, false, 0).next_attack, 42);
        // A slow weapon under 0.1 refuses the floor: step 0.01.
        let slow = windup(0.0, 100.0, 1000, 1.0, 0, false, 0);
        assert!((slow.step - 0.01).abs() < 1e-6, "{slow:?}");
    }

    /// A swing's balance gives the recovery's quadratic: balance 0.5,
    /// f = 0.71428575, (3 − 2f)f² = 0.8017492; the recovery is then 2.6035
    /// of the 110 ms base = 286 ms.
    #[test]
    fn a_poorly_balanced_swing_recovers_later() {
        let w = windup(0.5, 0.1, 1000, 0.5, 5, false, 0);
        assert!((w.recovery_factor - 0.801_749_2).abs() < 1e-5, "{w:?}");
        assert_eq!(w.recovery, 286);
        // Outside the leg modes' 5–12 the factor is 1.
        assert_eq!(
            windup(0.5, 0.1, 1000, 0.5, 4, false, 0).recovery_factor,
            1.0
        );
        assert_eq!(
            windup(0.5, 0.1, 1000, 0.5, 13, false, 0).recovery_factor,
            1.0
        );
        assert_eq!(windup(0.5, 0.1, 1000, 0.5, 5, true, 0).recovery_factor, 1.0);
    }

    /// The strongest engaged enemy: in front (its dot over 0.3·d), within
    /// reach plus 70 cm, and in the contact list with a score over 2.
    /// Since the contact list's scores stay under 2, the original finds
    /// none; the arithmetic is kept exactly.
    #[test]
    fn the_engaged_enemy_test_keeps_the_originals_threshold() {
        let player = Vec3::ZERO;
        let look = Vec3::Z;
        let f = [NearFighter {
            body: 1,
            at: Vec3::new(0.0, 0.0, 100.0),
            scale: 1.0,
            threat: 0.7,
        }];
        let inert = contacts(player, look, look, [(1, Vec3::new(0.0, 0.0, 100.0))]);
        assert_eq!(
            strongest_engaged(player, look, 60.0, &f, &inert),
            (false, 0.0)
        );
        let hot = [Contact {
            body: 1,
            score: 2.5,
            closeness: 0.9,
            facing: 1.0,
        }];
        assert_eq!(strongest_engaged(player, look, 60.0, &f, &hot), (true, 0.7));
        // Beside (dot 0) is refused by the 0.3·d test; closer than 1 cm too.
        let beside = [NearFighter {
            at: Vec3::new(100.0, 0.0, 0.0),
            ..f[0]
        }];
        assert!(!strongest_engaged(player, look, 200.0, &beside, &hot).0);
        let on = [NearFighter {
            at: Vec3::ZERO,
            ..f[0]
        }];
        assert!(!strongest_engaged(player, look, 200.0, &on, &hot).0);
    }
}
