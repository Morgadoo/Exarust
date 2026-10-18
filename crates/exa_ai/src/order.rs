//! An NPC's move, as the same fields the player writes: a heading and a
//! desired velocity on the character body.
//!
//! The path's carrot sits 100 cm ahead of the projection. The heading
//! moves a tenth of the way toward that direction. The body walks at the
//! player's walk speed once it is aligned, and at the player's jog speed
//! when the urgency is at least 1 and the alignment is past 0.9.
//! Evidence: the movement notes' NPC driver.

use bevy::math::Vec3;

/// The player's walk and jog, metres a second. An NPC writes these on the
/// same desired-velocity field the player does.
const WALK_SPEED: f32 = 1.4;
const JOG_SPEED: f32 = 3.6;

/// What one steer writes: the heading the body turns toward, and the
/// velocity the character body takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub heading: f32,
    pub velocity: Vec3,
}

/// The order that follows `path` from `at` (metres), starting from
/// `heading` (radians, facing +Z at 0). Urgency 0 walks. Urgency 1 or more
/// jogs once the heading is close to the carrot.
pub fn follow(path: &[Vec3], at: Vec3, heading: f32, urgency: u8) -> Order {
    if path.len() < 2 {
        return Order {
            heading,
            velocity: Vec3::ZERO,
        };
    }
    let Some(carrot) = carrot(path, at, 1.0) else {
        return Order {
            heading,
            velocity: Vec3::ZERO,
        };
    };
    let d = Vec3::new(carrot.x - at.x, 0.0, carrot.z - at.z);
    let dist = d.length();
    if dist < 1e-4 {
        return Order {
            heading,
            velocity: Vec3::ZERO,
        };
    }
    let dir = d / dist;
    let face = Vec3::new(heading.sin(), 0.0, heading.cos());
    let mut next = face + 0.1 * (dir - face);
    if next.length_squared() < 1e-8 {
        next = dir;
    }
    next = next.normalize();
    let align = dir.dot(face);
    let gate = (0.007 * dist).min(0.7);
    let moving = align > gate * 0.5;
    let speed = if !moving {
        0.0
    } else if urgency >= 1 && align > 0.9 {
        JOG_SPEED
    } else {
        WALK_SPEED
    };
    Order {
        heading: next.x.atan2(next.z),
        velocity: next * speed,
    }
}

/// The point `reach` metres further along `path` from the nearest point to
/// `at`.
fn carrot(path: &[Vec3], at: Vec3, reach: f32) -> Option<Vec3> {
    let mut best = f32::MAX;
    let mut along = 0.0;
    let mut cursor = 0.0;
    let mut found = 0.0;
    for window in path.windows(2) {
        let a = window[0];
        let b = window[1];
        let ab = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
        let len = ab.length();
        if len < 1e-6 {
            continue;
        }
        let t = ((at - a).dot(ab) / (len * len)).clamp(0.0, 1.0);
        let point = a + ab * t;
        let flat = Vec3::new(point.x - at.x, 0.0, point.z - at.z);
        let dist = flat.length();
        if dist < best {
            best = dist;
            found = cursor + len * t;
        }
        cursor += len;
        along = cursor;
    }
    if best == f32::MAX {
        return None;
    }
    let want = (found + reach).min(along);
    let mut left = want;
    for window in path.windows(2) {
        let a = window[0];
        let b = window[1];
        let ab = b - a;
        let len = Vec3::new(ab.x, 0.0, ab.z).length();
        if left <= len || len < 1e-6 {
            let t = if len < 1e-6 { 0.0 } else { left / len };
            return Some(a + ab * t);
        }
        left -= len;
    }
    path.last().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn following_a_straight_path_walks_at_the_players_speed() {
        let path = [Vec3::ZERO, Vec3::new(0.0, 0.0, 8.0)];
        let order = follow(&path, Vec3::ZERO, 0.0, 0);
        assert!((order.heading).abs() < 1e-4, "{}", order.heading);
        assert!(
            (order.velocity - Vec3::Z * WALK_SPEED).length() < 1e-4,
            "{}",
            order.velocity
        );
        let jog = follow(&path, Vec3::ZERO, 0.0, 1);
        assert!(
            (jog.velocity - Vec3::Z * JOG_SPEED).length() < 1e-4,
            "{}",
            jog.velocity
        );
    }

    #[test]
    fn a_path_of_one_point_does_not_move() {
        let order = follow(&[Vec3::ZERO], Vec3::ZERO, 0.4, 1);
        assert_eq!(order.velocity, Vec3::ZERO);
        assert!((order.heading - 0.4).abs() < 1e-6);
    }
}
