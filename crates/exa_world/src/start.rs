//! Where characters start in a level, as the original decides it: story
//! levels and the arena hub have fixed starts; arena fighters stand on a
//! ring around the arena's centre, each facing it. Evidence:
//! `docs/formats/RFC.md` (Starts).

use bevy::math::Vec3;

/// A position (metres, feet on the ground) and a facing: the character looks
/// along `(sin yaw, 0, cos yaw)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Start {
    pub position: Vec3,
    pub yaw: f32,
}

/// Arena centres (metres), by arena name.
const ARENA_CENTRES: [(&str, [f32; 3]); 12] = [
    ("arena1", [0.0, 0.0, 3.0]),
    ("arena2", [0.0, 0.0, 3.0]),
    ("arena3", [-3.0, 0.0, 3.0]),
    ("arena4", [-15.0, 0.0, -4.5]),
    ("arena5", [-3.0, 0.0, -3.0]),
    ("arena6", [3.0, 0.0, 3.0]),
    ("arena7", [0.0, 0.0, 0.0]),
    ("arenao1", [0.0, 0.0, 0.0]),
    ("arenao2", [0.0, 0.0, 0.0]),
    ("arenao3", [9.0, 0.0, 1.0]),
    ("arenao4", [0.0, 0.0, 0.0]),
    ("arenao5", [8.0, 0.0, 1.0]),
];

fn stem(level: &str) -> String {
    level
        .strip_suffix(".rfc")
        .unwrap_or(level)
        .to_ascii_lowercase()
}

/// Centre of an arena match, if `level` is an arena.
pub fn arena_centre(level: &str) -> Option<Vec3> {
    let stem = stem(level);
    ARENA_CENTRES
        .iter()
        .find(|(name, _)| *name == stem)
        .map(|(_, c)| Vec3::from_array(*c))
}

/// Fighters' starts in an arena: `teams` teams of `team_size`, on a ring
/// around `centre`, each facing the centre. Returns them per team.
pub fn arena_ring(centre: Vec3, teams: usize, team_size: usize) -> Vec<Vec<Start>> {
    let (n, k) = (teams.max(1) as f32, team_size.max(1) as f32);
    let radius = (0.5 * n * k + 1.0).min(3.5);
    let spread = 2.0 * (k - 1.0) / (k + 1.0);
    (0..teams.max(1))
        .map(|t| {
            let base = 6.0 * t as f32 / n - spread / 2.0;
            (0..team_size.max(1))
                .map(|m| {
                    let a = if team_size > 1 {
                        base + m as f32 * spread / (k - 1.0)
                    } else {
                        base
                    };
                    let theta = std::f32::consts::TAU * (3.0 - a) / 6.0;
                    let towards = Vec3::new(theta.sin(), 0.0, theta.cos());
                    Start {
                        position: centre - towards * radius,
                        yaw: towards.x.atan2(towards.z),
                    }
                })
                .collect()
        })
        .collect()
}

/// The player's start in `level`, where the original fixes it: a new game
/// in the first story level, the arena hub, the old practice arenas, and the
/// first slot of an arena match.
pub fn player_start(level: &str) -> Option<Start> {
    let stem = stem(level);
    let facing_minus_z = std::f32::consts::PI;
    match stem.as_str() {
        // A new game: the "unknown" background wakes facing (0.5, 0, −0.866).
        "exanima01" => Some(Start {
            position: Vec3::new(81.0, 0.0, 39.2),
            yaw: 150f32.to_radians(),
        }),
        "arenahub" => Some(Start {
            position: Vec3::new(0.0, 0.0, 3.0),
            yaw: facing_minus_z,
        }),
        "arena" | "arenae" => Some(Start {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: facing_minus_z,
        }),
        _ => arena_centre(&stem).map(|c| arena_ring(c, 2, 1)[0][0]),
    }
}

/// The opponent's start in a one-on-one arena match.
pub fn opponent_start(level: &str) -> Option<Start> {
    arena_centre(level).map(|c| arena_ring(c, 2, 1)[1][0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duel_faces_off_across_the_centre() {
        let c = Vec3::new(9.0, 0.0, 1.0);
        let ring = arena_ring(c, 2, 1);
        let (a, b) = (ring[0][0], ring[1][0]);
        assert!((a.position - (c + Vec3::new(0.0, 0.0, 2.0))).length() < 1e-4);
        assert!((b.position - (c - Vec3::new(0.0, 0.0, 2.0))).length() < 1e-4);
        // Each looks at the centre.
        let look = |s: Start| Vec3::new(s.yaw.sin(), 0.0, s.yaw.cos());
        assert!(look(a).dot((c - a.position).normalize()) > 0.999);
        assert!(look(b).dot((c - b.position).normalize()) > 0.999);
    }

    #[test]
    fn rings_grow_with_fighters_up_to_a_limit() {
        let r = |n, k| arena_ring(Vec3::ZERO, n, k)[0][0].position.length();
        assert!((r(2, 1) - 2.0).abs() < 1e-4);
        assert!((r(3, 3) - 3.5).abs() < 1e-4);
    }

    #[test]
    fn starts_by_level() {
        assert!(player_start("exanima01.rfc").is_some());
        assert!(player_start("arenao3").is_some());
        assert!(opponent_start("arenahub").is_none());
        assert!(player_start("exanima05").is_none());
    }
}
