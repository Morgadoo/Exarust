//! The player behaviour's own rules, outside its steering: the aim point's
//! soft height lock and raised surface, the gamepad aim's point and screen
//! ease, the near-contact list, the strongest engaged enemy, the character
//! nearest the cursor on screen and the attack timing. Rules:
//! `docs/PLAYER.md`.
//!
//! These are the parts of the original's player behaviour that are not the
//! locomotion request nor the cursor's follower ([`crate::steering`]), nor
//! the combat target and the stance ([`exa_combat::stance`]), nor the combat
//! swing (`exa_combat`). They are plain functions so that the arithmetic,
//! the comparisons and the branch order are pinned by tests; the running
//! game feeds the gamepad aim's point and ease ([`crate::player`]) and
//! the others are tested rules with no caller yet: the auto-target, the
//! look snap and the camera's follow step ([`auto_target`],
//! [`look_snaps`], [`camera_follow_yaw`]), the bone lock (the stance
//! keeps no target entity and no target bones are read to the screen) and
//! the attack timing [`windup`] (the running swing keeps its own stall and
//! renewal clocks in `exa_combat::swing`).
//!
//! Evidence: the behaviour's aim point, gamepad aim, contacts, engaged
//! enemy, screen-nearest character and attack timing, read in the
//! instructions (the tests below carry the numbers).

// NaN-sensitive compares are kept as the original makes them.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Vec2, Vec3};

/// The engine's single-precision epsilon, the floor under its "is it zero"
/// energy tests.
pub const EPSILON: f32 = 1.192_089_9e-7;
/// A character is in the player's contact list within 12 m (cm).
pub const CONTACT_RANGE_CM: f32 = 1200.0;
/// The contact list holds this many characters.
pub const CONTACTS: usize = 12;
/// A gamepad aim's point lies `250·|aim| + 50` cm out along it.
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
/// The strongest engaged enemy must lie within the player's reach (cm)
/// plus this, and in front of its heading by more than 0.3 of its distance
/// (within about 72.5°).
pub const ENGAGED_MARGIN_CM: f32 = 70.0;
pub const ENGAGED_COSINE: f64 = 0.3;
/// An enemy is engaged when its own contact score for the player is over
/// this.
pub const ENGAGED_SCORE: f32 = 2.0;
/// The minimum squared distance (cm²) at which the engaged test accepts a
/// character.
const SELF_RANGE2: f32 = 1.0;
/// The wind-up step's floors.
pub const WINDUP_FLOOR: f64 = 0.1;
pub const WINDUP_LEAST: f64 = 0.01;
/// The wind-up and recovery rates.
const WINDUP_PER_FRAME: f64 = 0.03;
const WINDUP_FRAMES: f32 = 80.0;
const WINDUP_MS: f32 = 1000.0;
const RECOVERY_BALANCE: f64 = 0.7;
/// The recovery's balance scale, 1/0.7 in double precision.
const RECOVERY_SLOPE: f64 = 1.428_571_428_571_428_6;
/// The screen-nearest search starts from this squared distance (px²): a
/// farther or unordered one is never taken.
const SCREEN_FAR2: f32 = 3.4e38;

/// A two-part value the behaviour keeps for frame interpolation: the
/// previous and current copies and the interpolated one the draw reads.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Interpolated2 {
    pub previous: Vec2,
    pub current: Vec2,
    pub shown: Vec2,
}

impl Interpolated2 {
    /// Sets all three copies to `at`, as the setter does.
    pub fn at(at: Vec2) -> Self {
        Self {
            previous: at,
            current: at,
            shown: at,
        }
    }

    /// The frame's advance: the previous copy takes the current one and
    /// the current takes `now`.
    pub fn advance(&mut self, now: Vec2) {
        self.previous = self.current;
        self.current = now;
    }

    /// The interpolated value at `t`: previous plus (current − previous)·t,
    /// without a clamp.
    pub fn lerp(&self, t: f32) -> Vec2 {
        self.previous + (self.current - self.previous) * t
    }
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

/// The aim point in the world after one tick of the gamepad ease (Exarust
/// eases the world point by the screen ease's factor).
pub fn aim_point_step(current: Vec3, want: Vec3, aim_speed: f32) -> Vec3 {
    current + (want - current) * aim_factor(aim_speed)
}

/// The aim height when a target's point shows near the cursor: `plane` is
/// the aim plane's height, `point` the height of the target's point the
/// lock blends toward (its head moved toward the nearest of its bones by
/// the bone's fraction), `distance2` that bone's squared screen distance to
/// the cursor and `radius` the pixel radius 80 cm make at the head's
/// depth. Within one radius the height is the point's; within two, the two
/// blend by `(d − r) / r`; beyond, the plane's.
pub fn aim_lock_height(plane: f32, point: f32, distance2: f32, radius: f32) -> f32 {
    let twice = radius * AIM_LOCK_RADII;
    if twice * twice > distance2 {
        if radius * radius >= distance2 {
            point
        } else {
            let blend = (distance2.sqrt() - radius) / radius;
            plane * blend + (1.0 - blend) * point
        }
    } else {
        plane
    }
}

/// A bone's projected point against the cursor, in screen pixels: from
/// `start` (the target's projected reference point) toward `end` (the
/// bone's), the fraction `t = (end − start)·(cursor − start) / |end −
/// start|²`, clamped into [0, 1] by the original's two ordered tests (a NaN
/// fraction fails both and stays NaN; −0 passes), and the squared distance
/// from `start + (end − start)·t` to the cursor, which the lock compares
/// with its radii.
pub fn bone_screen(start: Vec2, end: Vec2, cursor: Vec2) -> (f32, f32) {
    let bone = end - start;
    let to = cursor - start;
    let t = ((bone.x * to.x + bone.y * to.y) / (bone.x * bone.x + bone.y * bone.y)).clamp(0.0, 1.0);
    let d = start + bone * t - cursor;
    (t, d.x * d.x + d.y * d.y)
}

/// A point of a locked target the aim height can follow: where it shows on
/// screen (pixels) and its world height (cm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AimBone {
    pub screen: Vec2,
    pub height: f32,
}

/// The aim height with a locked target. Each of the target's bones, in
/// order, is measured through [`bone_screen`] from the target's reference
/// point `root`; the first bone is taken outright and a later one replaces
/// it only when strictly nearer (so an unordered distance never replaces a
/// nearer one, and a NaN first one is kept). The point the lock follows is
/// `root` moved toward the nearest bone by its fraction, `root + (bone −
/// root)·t` (the original lerps the two view-space points and maps the
/// result back to the world; its height is what is kept here), and
/// [`aim_lock_height`] blends it with the `plane`. Returns the height and
/// whether the bone was within two radii (the raised-surface rule is
/// skipped then).
pub fn aim_lock_bones(
    plane: f32,
    root: AimBone,
    bones: &[AimBone],
    cursor: Vec2,
    radius: f32,
) -> (f32, bool) {
    let mut nearest: Option<(f32, f32)> = None;
    for bone in bones {
        let (t, d2) = bone_screen(root.screen, bone.screen, cursor);
        if nearest.is_none_or(|(_, best)| d2 < best) {
            nearest = Some((root.height + (bone.height - root.height) * t, d2));
        }
    }
    let Some((point, d2)) = nearest else {
        return (plane, false);
    };
    let twice = radius * AIM_LOCK_RADII;
    (
        aim_lock_height(plane, point, d2, radius),
        twice * twice > d2,
    )
}

/// The pixel radius 80 cm makes at a point `depth` cm in front of the eye:
/// the viewport's width times the projection's scale, halved, times 80,
/// over the depth. The original takes the viewport's own pixel width and
/// the projection's scale (the same terms Bevy's perspective projection
/// gives as `1 / tan(fov_y / 2)`).
pub fn bone_lock_radius(viewport_px: f32, projection_scale: f32, depth: f32) -> f32 {
    viewport_px * projection_scale * 0.5 * 80.0 / depth
}

/// The aim point without a lock: the cursor's point on the aim plane, or the
/// point the cursor's pick met (`hit`, not the player's own body) when it
/// lies above the plane's point and below the player's ground plus 200 cm;
/// the whole hit point is taken.
pub fn aim_raised(plane_point: Vec3, hit: Option<Vec3>, ground: f32) -> Vec3 {
    match hit {
        Some(hit) if hit.y > plane_point.y && ground + RAISED_SURFACE_MAX_CM > hit.y => hit,
        _ => plane_point,
    }
}

/// A character near the player, as the player's contact list keeps it: the
/// body's id, the closeness (up to 0.99), the facing weight and their
/// combined score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub body: u64,
    pub score: f32,
    pub closeness: f32,
    pub facing: f32,
}

/// Builds the player's contact list, rebuilt each tick over every character
/// within 12 m (`at` the positions, cm, compared across the ground): the
/// closeness falls from 1 by `d · 0.00083333335` (d in cm) and stops at
/// 0.99; the facing weight is `0.75 · (0.5 − 0.5·facing·dir) + 0.25 ·
/// (0.5 − 0.5·cursor·dir)²` with `facing` the body's forward axis and
/// `cursor` the cursor's direction; the score is half of each. The list
/// holds twelve: a new character takes the place of the lowest scoring one
/// (the first of equals) when it scores higher.
pub fn contacts(
    player: Vec3,
    facing: Vec3,
    cursor: Vec3,
    others: impl IntoIterator<Item = (u64, Vec3)>,
) -> Vec<Contact> {
    let mut out: Vec<Contact> = Vec::with_capacity(CONTACTS);
    let dot = |a: Vec3, b: Vec3| (a.y * b.y + a.x * b.x) + a.z * b.z;
    for (body, at) in others {
        let mut delta = at - player;
        delta.y = 0.0;
        let d2 = (delta.y * delta.y + delta.x * delta.x) + delta.z * delta.z;
        if !(d2 < CONTACT_RANGE_CM * CONTACT_RANGE_CM) {
            continue;
        }
        let d = d2.sqrt();
        let dir = delta * (1.0 / d);
        let mut closeness = 1.0 - d * 0.000_833_333_35;
        if f64::from(closeness) > 0.99 {
            closeness = 0.99;
        }
        let ahead = dot(facing, dir) * -0.5 + 0.5;
        let cursor_part = dot(cursor, dir) * -0.5 + 0.5;
        let facing_part = (cursor_part * cursor_part) * 0.25 + ahead * 0.75;
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
            let mut lowest = 0;
            for i in 1..out.len() {
                if out[lowest].score > out[i].score {
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

/// A character near the player, as the strongest-engaged test reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearFighter {
    /// Its position (cm).
    pub at: Vec3,
    /// Its threat, the value the test keeps the largest of.
    pub threat: f32,
    /// Its own contact score for the player, when the player is in its
    /// contact list.
    pub player_score: Option<f32>,
    /// Collapsing or dead, or kept out of the test by its behaviour.
    pub skipped: bool,
}

/// Whether an enemy is engaged with the player: the player is in its own
/// contact list with a score over 2.
pub fn engaged(player_score: Option<f32>) -> bool {
    player_score.is_some_and(|s| s > ENGAGED_SCORE)
}

/// An opponent the auto-target considers: where it stands (cm), whether
/// the view can see it and whether it is out of the scene (flag 0x800).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AimCandidate {
    pub at: Vec3,
    pub visible: bool,
    pub skipped: bool,
}

/// The auto-target's reach (cm): the squared 250 000 of the original's
/// 500 cm.
pub const AUTO_TARGET_CM: f32 = 500.0;

/// The auto-target: the visible opponent within 500 cm whose score
/// `(500 − d)·(δ·cursor/d + 0.5)` is the highest, strictly over the best so
/// far (the ordered compare, so a NaN never wins), the first on a tie.
/// `player` is the player's chest point and `cursor` its direction to the
/// cursor, both in the body's frame as the original takes them. Returns its
/// index.
pub fn auto_target(
    player: Vec3,
    cursor: Vec3,
    others: impl IntoIterator<Item = AimCandidate>,
) -> Option<usize> {
    // The original keeps the best score from zero here, so a score that is
    // zero, negative or NaN never becomes the target.
    let mut best = 0.0f32;
    let mut found = None;
    for (index, other) in others.into_iter().enumerate() {
        if other.skipped || !other.visible {
            continue;
        }
        let delta = other.at - player;
        let d2 = (delta.y * delta.y + delta.x * delta.x) + delta.z * delta.z;
        // The reach test skips only an ordered "beyond": a NaN passes on
        // to a NaN score, which never wins.
        if AUTO_TARGET_CM * AUTO_TARGET_CM < d2 {
            continue;
        }
        let d = d2.sqrt();
        let dot = (delta.y * cursor.y + delta.x * cursor.x) + delta.z * cursor.z;
        let score = (AUTO_TARGET_CM - d) * (dot / d + 0.5);
        if best < score {
            best = score;
            found = Some(index);
        }
    }
    found
}

/// The camera's follow step (the behaviour's per-tick call when the
/// camera follows the body): `camera` is the camera's present yaw and
/// `body` the yaw of the body's direction. With the signed gap `camera −
/// body` wrapped into ±π (compared in double precision) more than 0.1π
/// (the single 0.1 times π, in double) away, the camera's yaw becomes the
/// body's yaw put back 0.1π toward the camera's side (the step's sign is
/// the gap's), wrapped again; within it, nothing changes (`None`). A NaN
/// fails the ordered compare and changes nothing.
pub fn camera_follow_yaw(camera: f32, body: f32) -> Option<f32> {
    let gap = exa_ai::combat::gap(camera, body);
    let step = f64::from(CAMERA_FOLLOW_STEP) * std::f64::consts::PI;
    if !(step < f64::from(gap.abs())) {
        return None;
    }
    let sign = if gap.is_sign_negative() { -1.0 } else { 1.0 };
    Some(exa_ai::combat::wrap(body + (sign * step) as f32))
}

/// The camera follow's step in half turns (times π): 0.1.
const CAMERA_FOLLOW_STEP: f32 = 0.1;

/// Whether the look snaps to the cursor's direction this tick: the gap
/// `look − cursor` wrapped into ±π (compared in double precision), its
/// magnitude strictly over 30° (the single 30 times the single degree).
/// The original then copies the behaviour's cursor direction vector into
/// the look. A NaN fails the ordered compare: no snap.
pub fn look_snaps(look: f32, cursor: f32) -> bool {
    SNAP_LOOK < exa_ai::combat::gap(look, cursor).abs()
}

/// The cone the look snaps beyond: 30°, the product of two singles.
const SNAP_LOOK: f32 = 30.0 * 0.017_453_292;

/// The strongest engaged enemy in front within the player's reach plus
/// 70 cm (`from` the player's point and `heading` its heading, cm): the
/// squared distance within the reach's and at least 1, the dot with the
/// heading not negative and over 0.3 of the distance (in double
/// precision), and the enemy engaged. Every distance and dot test lets an
/// unordered value through, as the original's do. Returns whether one was
/// found and the largest threat among them (from 0).
pub fn strongest_engaged(
    from: Vec3,
    heading: Vec3,
    reach: f32,
    fighters: &[NearFighter],
) -> (bool, f32) {
    let mut found = false;
    let mut best = 0.0f32;
    let limit = reach + ENGAGED_MARGIN_CM;
    for f in fighters {
        if f.skipped {
            continue;
        }
        let delta = f.at - from;
        let d2 = (delta.y * delta.y + delta.x * delta.x) + delta.z * delta.z;
        if limit * limit < d2 || d2 < SELF_RANGE2 {
            continue;
        }
        let dot = (delta.y * heading.y + delta.x * heading.x) + delta.z * heading.z;
        if dot < 0.0 {
            continue;
        }
        let d = d2.sqrt();
        if !(f64::from(d) * ENGAGED_COSINE < f64::from(dot)) {
            continue;
        }
        if !engaged(f.player_score) {
            continue;
        }
        if f.threat > best {
            best = f.threat;
        }
        found = true;
    }
    (found, best)
}

/// The character closest to the cursor on screen: among the characters not
/// dead, the one whose point shows nearest the cursor (squared pixels,
/// `(dy)² + (dx)²`), strictly nearer than any before it and than 3.4e38.
pub fn closest_to_cursor(
    cursor: Vec2,
    others: impl IntoIterator<Item = (u64, Vec2, bool)>,
) -> Option<u64> {
    let mut best: Option<u64> = None;
    let mut best_d2 = SCREEN_FAR2;
    for (body, screen, dead) in others {
        if dead {
            continue;
        }
        let dx = screen.x - cursor.x;
        let dy = screen.y - cursor.y;
        let d2 = dy * dy + dx * dx;
        if best_d2 > d2 {
            best_d2 = d2;
            best = Some(body);
        }
    }
    best
}

/// The attack timing after a swing's tick: its wind-up step, the game clock
/// (ms) of the next attack and of the recovery. `energy` is the fighter's
/// raw energy, `rate` its primary weapon's recoil (W17, the record field
/// the original divides by), `swing_frames` its swing's
/// expected length, `balance` its balance, `mode` its attack motion's
/// number (5 to 12 being a swing's) and `no_recovery` the flag that skips
/// the recovery's quadratic.
pub fn windup(
    energy: f32,
    rate: f32,
    swing_frames: u32,
    balance: f32,
    mode: i32,
    no_recovery: bool,
    now_ms: u32,
) -> Windup {
    let mut step = if energy > EPSILON {
        (1.0 - f64::from(energy.sqrt()) * 0.05) as f32
    } else {
        1.0
    };
    if f64::from(step) < WINDUP_FLOOR {
        step = WINDUP_FLOOR as f32;
    }
    step = (f64::from(step / rate) * WINDUP_PER_FRAME) as f32;
    if f64::from(step) < WINDUP_LEAST {
        step = WINDUP_LEAST as f32;
    }
    // The engine converts to an integer with the nearest-even mode.
    let by_step = (WINDUP_MS / (WINDUP_FRAMES * step)).round_ties_even() as u32;
    let frames = by_step.min(swing_frames);
    let next_attack = now_ms.wrapping_add(frames);
    let factor =
        if f64::from(balance) > RECOVERY_BALANCE || !(5..=12).contains(&mode) || no_recovery {
            1.0
        } else {
            let f = (f64::from(balance) * RECOVERY_SLOPE) as f32;
            (3.0 - f * 2.0) * (f * f)
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

#[cfg(test)]
mod tests {
    use super::*;

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
        // At 2.25 the factor is one: it arrives at once.
        let fast = aim_cursor_step(Vec2::ZERO, want, 2.25);
        assert!((fast - want).length() < 1e-4);
        // The world point eases by the same factor.
        let p = aim_point_step(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 0.29);
        assert!((p.x - 4.6656).abs() < 1e-4, "{p}");
        // The point is 250·|aim| + 50 cm out.
        assert_eq!(aim_distance(0.0), 50.0);
        assert_eq!(aim_distance(1.0), 300.0);
    }

    /// A bone's projection gives the fraction along it and the squared
    /// distance to the cursor: 4 px beside its middle is 16 px².
    #[test]
    fn the_bone_screen_fraction_and_distance() {
        let (t, d2) = bone_screen(
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 100.0),
            Vec2::new(14.0, 50.0),
        );
        assert!((t - 0.5).abs() < 1e-6);
        assert!((d2 - 16.0).abs() < 1e-6);
        // Beyond the ends the fraction clamps.
        let (t, _) = bone_screen(Vec2::ZERO, Vec2::X * 10.0, Vec2::new(20.0, 0.0));
        assert_eq!(t, 1.0);
        let (t, _) = bone_screen(Vec2::ZERO, Vec2::X * 10.0, Vec2::new(-5.0, 0.0));
        assert_eq!(t, 0.0);
        // A zero-length bone's division is NaN, which fails both ordered
        // tests and stays NaN.
        assert!(bone_screen(Vec2::ZERO, Vec2::ZERO, Vec2::X).0.is_nan());
    }

    /// An aim point is fully locked within one cursor radius (its edge
    /// included), blends by (d − r)/r to two radii, and keeps the plane from
    /// two radii on.
    #[test]
    fn the_aim_locks_onto_a_bone_within_two_radii() {
        assert_eq!(aim_lock_height(0.8, 1.5, 25.0, 10.0), 1.5);
        assert_eq!(aim_lock_height(0.8, 1.5, 100.0, 10.0), 1.5);
        // 15 px: blend (15 − 10)/10 = 0.5.
        assert!((aim_lock_height(0.8, 1.5, 225.0, 10.0) - 1.15).abs() < 1e-5);
        assert_eq!(aim_lock_height(0.8, 1.5, 400.0, 10.0), 0.8);
    }

    /// The bone lock follows the reference point moved toward the nearest
    /// bone by its fraction, locks to that point's height within one
    /// radius, blends by (d − r)/r to two radii and keeps the plane beyond;
    /// an unordered bone never replaces a nearer one. The pixel radius of
    /// 80 cm is width·scale·0.5·80 over the depth.
    #[test]
    fn the_aim_lock_follows_the_point_toward_the_nearest_bone() {
        let root = AimBone {
            screen: Vec2::new(100.0, 100.0),
            height: 100.0,
        };
        let far = AimBone {
            screen: Vec2::new(300.0, 300.0),
            height: 120.0,
        };
        // A bone straight up the screen from the root, 200 px long, 60 cm
        // higher in the world.
        let up = AimBone {
            screen: Vec2::new(100.0, 300.0),
            height: 160.0,
        };
        // The cursor at the bone's middle: t = 0.5, distance 0, so the
        // point is halfway up, 130 cm, fully locked.
        let cursor = Vec2::new(100.0, 200.0);
        assert_eq!(
            aim_lock_bones(80.0, root, &[far, up], cursor, 50.0),
            (130.0, true)
        );
        // 30 px to the right of the bone's middle, with a 20 px radius:
        // (30 − 20)/20 = 0.5 between the plane (80) and the point (130).
        let cursor = Vec2::new(130.0, 200.0);
        let (height, locked) = aim_lock_bones(80.0, root, &[up], cursor, 20.0);
        assert!(locked);
        assert!((height - 105.0).abs() < 1e-4, "{height}");
        // With a 10 px radius 30 px is past two radii: the plane, unlocked.
        assert_eq!(
            aim_lock_bones(80.0, root, &[up], cursor, 10.0),
            (80.0, false)
        );
        // A later bone replaces the first only when strictly nearer.
        let (height, _) = aim_lock_bones(80.0, root, &[up, up], Vec2::new(100.0, 200.0), 50.0);
        assert_eq!(height, 130.0);
        // A broken (NaN) first bone is kept: no later bone is nearer than
        // NaN, and NaN is not within two radii, so the plane stays.
        let broken = AimBone {
            screen: Vec2::new(f32::NAN, 0.0),
            height: 300.0,
        };
        assert_eq!(
            aim_lock_bones(80.0, root, &[broken, up], Vec2::new(100.0, 200.0), 50.0),
            (80.0, false)
        );
        // No bones at all: the plane.
        assert_eq!(aim_lock_bones(80.0, root, &[], cursor, 50.0), (80.0, false));
        // 1000 px across, scale 1, 800 cm deep: 50 px.
        assert_eq!(bone_lock_radius(1000.0, 1.0, 800.0), 50.0);
    }

    /// The auto-target keeps the visible opponent within 500 cm whose
    /// `(500 − d)(cos + 0.5)` is highest: a nearer one ahead beats a farther
    /// one, one behind scores negative, and an invisible, skipped or far one
    /// never wins.
    #[test]
    fn the_auto_target_scores_by_distance_and_cursor() {
        let others = [
            AimCandidate {
                at: Vec3::new(0.0, 0.0, 300.0),
                visible: true,
                skipped: false,
            },
            AimCandidate {
                at: Vec3::new(0.0, 0.0, 100.0),
                visible: true,
                skipped: false,
            },
            AimCandidate {
                at: Vec3::new(0.0, 0.0, -100.0),
                visible: true,
                skipped: false,
            },
        ];
        // A at 300 scores (200)(1.5) = 300; the nearer B (400)(1.5) = 600;
        // C behind (400)(-0.5) = -200.
        assert_eq!(auto_target(Vec3::ZERO, Vec3::Z, others), Some(1));
        // Beyond 500 cm, invisible, or out of the scene: none.
        for other in [
            AimCandidate {
                at: Vec3::new(0.0, 0.0, 501.0),
                visible: true,
                skipped: false,
            },
            AimCandidate {
                at: Vec3::new(0.0, 0.0, 100.0),
                visible: false,
                skipped: false,
            },
            AimCandidate {
                at: Vec3::new(0.0, 0.0, 100.0),
                visible: true,
                skipped: true,
            },
        ] {
            assert_eq!(auto_target(Vec3::ZERO, Vec3::Z, [other]), None);
        }
        // Exactly 500 cm is in reach, but its score is 0: not over the
        // zero the best starts from.
        assert_eq!(
            auto_target(
                Vec3::ZERO,
                Vec3::Z,
                [AimCandidate {
                    at: Vec3::new(0.0, 0.0, 500.0),
                    visible: true,
                    skipped: false,
                }]
            ),
            None
        );
        // A NaN direction never wins: no candidate.
        assert_eq!(
            auto_target(
                Vec3::ZERO,
                Vec3::new(f32::NAN, 0.0, 1.0),
                [AimCandidate {
                    at: Vec3::new(0.0, 0.0, 100.0),
                    visible: true,
                    skipped: false,
                }]
            ),
            None
        );
    }

    /// The camera follows the body: over 0.1π from the body's yaw, it is
    /// put back to the body's yaw 0.1π on its own side (wrapped); within,
    /// nothing changes. The look snaps once the gap is strictly over 30°.
    #[test]
    fn the_camera_follow_and_the_look_snap() {
        let step = f32::from_bits(0x3ea0_d97c);
        assert_eq!(camera_follow_yaw(1.0, 0.0), Some(step));
        assert_eq!(camera_follow_yaw(-1.0, 0.0), Some(-step));
        assert_eq!(camera_follow_yaw(0.3, 0.0), None);
        // −3.0 and 3.0 are 0.283 apart the short way: within the step.
        assert_eq!(camera_follow_yaw(-3.0, 3.0), None);
        // 2.9 is 0.383 the short way below −3.0 (gap wrapped negative):
        // −3.0 − 0.1π = −3.3141594 wraps to 2.969026.
        let got = camera_follow_yaw(2.9, -3.0).unwrap();
        assert!((got - 2.969_026).abs() < 1e-5, "{got}");
        assert_eq!(camera_follow_yaw(0.5, f32::NAN), None);
        assert_eq!(SNAP_LOOK, f32::from_bits(0x3f06_0a92));
        assert!(look_snaps(0.0, 0.53));
        assert!(look_snaps(0.53, 0.0));
        assert!(!look_snaps(0.0, 0.52));
        assert!(!look_snaps(0.0, SNAP_LOOK));
        // 3.0 and −3.0 are 0.283 apart across ±π.
        assert!(!look_snaps(3.0, -3.0));
        assert!(!look_snaps(f32::NAN, 0.0));
    }

    /// A raised surface under the cursor replaces the aim point when it is
    /// above the plane's point and under the player's ground plus 200 cm.
    #[test]
    fn a_picked_surface_raises_the_aim_point() {
        let plane = Vec3::new(10.0, 80.0, 20.0);
        let table = Vec3::new(12.0, 120.0, 25.0);
        assert_eq!(aim_raised(plane, Some(table), 0.0), table);
        assert_eq!(aim_raised(plane, Some(table.with_y(70.0)), 0.0), plane);
        assert_eq!(aim_raised(plane, Some(table.with_y(200.0)), 0.0), plane);
        assert_eq!(
            aim_raised(plane, Some(table.with_y(240.0)), 50.0),
            table.with_y(240.0)
        );
        assert_eq!(aim_raised(plane, None, 0.0), plane);
    }

    /// The contacts hold twelve bodies; closeness falls 0.00083333335 a cm
    /// from 1, capped at 0.99; the facing weight is three quarters the
    /// facing's and a quarter the cursor's squared; the score is half the
    /// facing weight and half the closeness.
    #[test]
    fn the_contact_list_weighs_closeness_and_facing() {
        let here = Vec3::ZERO;
        let forward = Vec3::Z;
        // 100 cm ahead of both: closeness 1 − 0.083333335; both weights 0.
        let c = contacts(here, forward, forward, [(1, Vec3::new(0.0, 0.0, 100.0))])[0];
        assert!((c.closeness - 0.916_666_7).abs() < 1e-6, "{c:?}");
        assert!(c.facing.abs() < 1e-6, "{c:?}");
        assert!((c.score - 0.458_333_35).abs() < 1e-6, "{c:?}");
        // Behind both: 0.75 + 0.25.
        let behind = contacts(here, forward, forward, [(1, Vec3::new(0.0, 0.0, -100.0))])[0];
        assert!((behind.facing - 1.0).abs() < 1e-6, "{behind:?}");
        // Behind the body, ahead of the cursor: 0.75; the other way round
        // 0.25 (the cursor's part squared).
        let c = contacts(here, -forward, forward, [(1, Vec3::new(0.0, 0.0, 100.0))])[0];
        assert!((c.facing - 0.75).abs() < 1e-6, "{c:?}");
        let c = contacts(here, forward, -forward, [(1, Vec3::new(0.0, 0.0, 100.0))])[0];
        assert!((c.facing - 0.25).abs() < 1e-6, "{c:?}");
        // Beside the cursor: (0.5)²·0.25 = 0.0625.
        let c = contacts(here, forward, Vec3::X, [(1, Vec3::new(0.0, 0.0, 100.0))])[0];
        assert!((c.facing - 0.0625).abs() < 1e-6, "{c:?}");
        // 12 m is the end of the list: it is not held.
        assert!(contacts(here, forward, forward, [(1, Vec3::Z * 1200.0)]).is_empty());
        assert_eq!(
            contacts(here, forward, forward, [(1, Vec3::Z * 1199.0)]).len(),
            1
        );
        // Thirteen bodies ahead: the thirteenth, farthest, scores lowest and
        // stays out.
        let many: Vec<_> = (0..13)
            .map(|i| (i as u64, Vec3::new(0.0, 0.0, 100.0 + i as f32)))
            .collect();
        let kept = contacts(here, forward, forward, many);
        assert_eq!(kept.len(), 12);
        assert!(kept.iter().all(|c| c.body != 12), "{kept:?}");
        // A nearer thirteenth takes the lowest's place.
        let mut many: Vec<_> = (0..12)
            .map(|i| (i as u64, Vec3::new(0.0, 0.0, 100.0 + i as f32)))
            .collect();
        many.push((12, Vec3::new(0.0, 0.0, 50.0)));
        let kept = contacts(here, forward, forward, many);
        assert!(kept.iter().any(|c| c.body == 12) && kept.iter().all(|c| c.body != 11));
    }

    /// The strongest engaged enemy: in front (its dot over 0.3·d), within
    /// reach plus 70 cm, and the player in its own contacts with a score
    /// over 2; the largest threat wins.
    #[test]
    fn the_engaged_enemy_is_in_front_within_reach_and_engaged() {
        let player = Vec3::ZERO;
        let heading = Vec3::Z;
        let f = NearFighter {
            at: Vec3::new(0.0, 0.0, 100.0),
            threat: 0.7,
            player_score: Some(2.5),
            skipped: false,
        };
        assert_eq!(strongest_engaged(player, heading, 60.0, &[f]), (true, 0.7));
        // Beyond 60 + 70 cm it is out of reach.
        let far = NearFighter {
            at: Vec3::new(0.0, 0.0, 130.5),
            ..f
        };
        assert!(!strongest_engaged(player, heading, 60.0, &[far]).0);
        // Not engaged: its score for the player is 2 or less, or it has none.
        let calm = NearFighter {
            player_score: Some(2.0),
            ..f
        };
        assert!(!strongest_engaged(player, heading, 60.0, &[calm]).0);
        let unaware = NearFighter {
            player_score: None,
            ..f
        };
        assert!(!strongest_engaged(player, heading, 60.0, &[unaware]).0);
        // Beside (dot 0) is refused by the 0.3·d test; closer than 1 cm too;
        // a dead one is skipped.
        let beside = NearFighter {
            at: Vec3::new(100.0, 0.0, 0.0),
            ..f
        };
        assert!(!strongest_engaged(player, heading, 200.0, &[beside]).0);
        let on = NearFighter {
            at: Vec3::ZERO,
            ..f
        };
        assert!(!strongest_engaged(player, heading, 200.0, &[on]).0);
        let dead = NearFighter { skipped: true, ..f };
        assert!(!strongest_engaged(player, heading, 200.0, &[dead]).0);
        // The largest threat of two; a found enemy with no threat keeps 0.
        let other = NearFighter { threat: 0.9, ..f };
        assert_eq!(
            strongest_engaged(player, heading, 60.0, &[f, other]),
            (true, 0.9)
        );
        let none = NearFighter { threat: 0.0, ..f };
        assert_eq!(
            strongest_engaged(player, heading, 60.0, &[none]),
            (true, 0.0)
        );
    }

    /// The character closest to the cursor's screen point, the dead left
    /// out; an unordered distance is never taken.
    #[test]
    fn the_screen_nearest_character() {
        let got = closest_to_cursor(
            Vec2::new(100.0, 100.0),
            [
                (1, Vec2::new(150.0, 150.0), false),
                (2, Vec2::new(110.0, 110.0), false),
                (3, Vec2::new(101.0, 101.0), true),
            ],
        );
        assert_eq!(got, Some(2));
        assert_eq!(
            closest_to_cursor(Vec2::ZERO, [(1, Vec2::new(f32::NAN, 0.0), false)]),
            None
        );
    }

    /// The wind-up: 1 − 0.05·√E (1 when the energy is zero), floored at 0.1,
    /// over the weapon's rate, times 0.03, floored at 0.01; the next attack
    /// is 1000/(80·step) ms on, at most the swing's own length.
    #[test]
    fn the_attack_timing_drains_from_the_wind_up() {
        let w = windup(1.0, 0.1, 1000, 1.0, 0, false, 10_000);
        // step = 0.95/0.1·0.03 = 0.285, so 1000/(80·0.285) = 43.86 → 44.
        assert!((w.step - 0.285).abs() < 1e-5, "{w:?}");
        assert_eq!(w.next_attack, 10_044);
        // The recovery factor 1: (100·0.1 + 100)·3 = 330 ms.
        assert_eq!(w.recovery, 10_330);
        // A swing only ten frames long caps the wait.
        assert_eq!(windup(1.0, 0.1, 10, 1.0, 0, false, 0).next_attack, 10);
        // Zero energy takes the 1.0 step: 1000/(80·0.3) = 41.67 → 42.
        assert_eq!(windup(0.0, 0.1, 1000, 1.0, 0, false, 0).next_attack, 42);
        // A slow weapon's step stops at 0.01.
        let slow = windup(0.0, 100.0, 1000, 1.0, 0, false, 0);
        assert!((slow.step - 0.01).abs() < 1e-6, "{slow:?}");
    }

    /// The two-part value: setting puts all three copies on the same point,
    /// advancing shifts previous←current and current←now, and the shown
    /// value is previous + (current − previous)·t.
    #[test]
    fn the_two_part_value_interpolates() {
        let mut v = Interpolated2::at(Vec2::new(2.0, 3.0));
        assert_eq!(
            (v.previous, v.current, v.shown),
            (v.current, v.current, v.current)
        );
        v.advance(Vec2::new(6.0, 9.0));
        assert_eq!(v.previous, Vec2::new(2.0, 3.0));
        assert_eq!(v.current, Vec2::new(6.0, 9.0));
        assert_eq!(v.lerp(0.5), Vec2::new(4.0, 6.0));
        assert_eq!(v.lerp(-1.0), Vec2::new(-2.0, -3.0));
        assert_eq!(
            Interpolated2 {
                previous: Vec2::ZERO,
                current: Vec2::ZERO,
                shown: Vec2::ZERO,
            },
            Interpolated2::default()
        );
    }

    /// A swing's balance gives the recovery's quadratic: balance 0.5,
    /// f = 0.5/0.7 = 0.71428573, (3 − 2f)·f² = 0.8017492; the recovery is
    /// then 2.6034985 of the 110 ms base = 286 ms.
    #[test]
    fn a_poorly_balanced_swing_recovers_later() {
        let w = windup(0.5, 0.1, 1000, 0.5, 5, false, 0);
        assert_eq!(w.recovery_factor, 0.801_749_2, "{w:?}");
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
        // Balance over 0.7 (in double precision) too.
        assert_eq!(
            windup(0.5, 0.1, 1000, 0.71, 5, false, 0).recovery_factor,
            1.0
        );
    }
}
