//! The player controls' script hooks: the near scripted objects a drag or
//! use reaches, the scripts a dead character's roles run when the player
//! interacts with it, and the condition a living character's talk option
//! must pass. These are the script-side pieces of the interact dispatcher
//! (`docs/PLAYER.md`, Using what the cursor is over).
//!
//! The original walks every scene object, measures the **squared** distance
//! (its vector-distance helper returns the square) and compares it with 5,
//! so an object is "near" within √5 cm — a quirk kept. A role's script
//! runs with the dead character and the player as its context.

use bevy::math::Vec3;

/// The squared distance under which the original calls a near object's
/// script (5.0 cm², about 2.24 cm).
pub const NEAR_DISTANCE2: f32 = 5.0;

/// A scene object's script hook as the near walk reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptObject {
    pub at: Vec3,
    /// The object's script instance; `None` skips it.
    pub script: Option<u64>,
    /// Whether the script instance has an owner (its class word is set).
    pub has_owner: bool,
}

/// The scripts of the objects within √5 cm of `point`, in the scene's
/// order (F75). The original calls each one with the player; the ids are
/// returned here for the caller to run.
pub fn nearby_scripts(point: Vec3, objects: &[ScriptObject]) -> Vec<u64> {
    let mut out = Vec::new();
    for object in objects {
        let delta = object.at - point;
        let d2 = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;
        // Metres to the original's centimetres, squared.
        if d2 * 10_000.0 < NEAR_DISTANCE2
            && let Some(script) = object.script
            && object.has_owner
        {
            out.push(script);
        }
    }
    out
}

/// One role of a dead character and the script that role offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleScript {
    pub role: u32,
    pub script: Option<u64>,
}

/// The corpse interaction's calls (F112): every role with a script is run
/// with `(dead, player)` as its context, in role order.
pub fn corpse_scripts(roles: &[RoleScript]) -> Vec<(u32, u64)> {
    roles
        .iter()
        .filter_map(|r| r.script.map(|s| (r.role, s)))
        .collect()
}

/// The dialogue option the role relation engine returns for a talk
/// attempt: its status word and its script's length and id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TalkOption {
    pub status: i32,
    pub script: Option<u64>,
}

/// What a talk attempt did (F113).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalkOutcome {
    /// No option: the character cannot be talked to.
    Refused,
    /// Status 0: the interaction is allowed with no dialogue script.
    Allowed,
    /// The option's script ran and answered: allowed when its result is
    /// over zero.
    Scripted { allowed: bool, script: u64 },
}

/// The talk condition: no option refuses; status 0 allows; otherwise the
/// option's script runs and its positive result allows. `run` returns the
/// script's result.
pub fn talk(option: Option<TalkOption>, run: impl FnOnce(u64) -> i32) -> TalkOutcome {
    let Some(option) = option else {
        return TalkOutcome::Refused;
    };
    if option.status == 0 {
        return TalkOutcome::Allowed;
    }
    let Some(script) = option.script else {
        return TalkOutcome::Scripted {
            allowed: false,
            script: 0,
        };
    };
    TalkOutcome::Scripted {
        allowed: run(script) > 0,
        script,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only objects within √5 cm with an owned script are called, in scene
    /// order.
    #[test]
    fn the_near_scripts_use_the_squared_five_centimetre_radius() {
        let point = Vec3::ZERO;
        let objects = [
            ScriptObject {
                at: Vec3::new(0.02, 0.0, 0.0),
                script: Some(7),
                has_owner: true,
            },
            ScriptObject {
                at: Vec3::new(0.05, 0.0, 0.0),
                script: Some(8),
                has_owner: true,
            },
            ScriptObject {
                at: Vec3::new(0.01, 0.0, 0.0),
                script: None,
                has_owner: true,
            },
            ScriptObject {
                at: Vec3::new(0.01, 0.0, 0.0),
                script: Some(9),
                has_owner: false,
            },
        ];
        // 2 cm is inside (4 < 5); 5 cm is not (25 > 5).
        assert_eq!(nearby_scripts(point, &objects), vec![7]);
        let edge = [ScriptObject {
            at: Vec3::new(0.0, (NEAR_DISTANCE2 - 0.001).sqrt() / 100.0, 0.0),
            script: Some(3),
            has_owner: true,
        }];
        assert_eq!(nearby_scripts(point, &edge), vec![3]);
    }

    /// A dead character's roles with scripts all run, in role order.
    #[test]
    fn the_corpse_runs_every_role_with_a_script() {
        let roles = [
            RoleScript {
                role: 4,
                script: Some(40),
            },
            RoleScript {
                role: 5,
                script: None,
            },
            RoleScript {
                role: 6,
                script: Some(60),
            },
        ];
        assert_eq!(corpse_scripts(&roles), vec![(4, 40), (6, 60)]);
        assert!(corpse_scripts(&[]).is_empty());
    }

    /// No option refuses, status 0 allows, else the script's positive
    /// result allows.
    #[test]
    fn the_talk_condition() {
        assert_eq!(talk(None, |_| 1), TalkOutcome::Refused);
        assert_eq!(
            talk(
                Some(TalkOption {
                    status: 0,
                    script: None
                }),
                |_| 0
            ),
            TalkOutcome::Allowed
        );
        assert_eq!(
            talk(
                Some(TalkOption {
                    status: 7,
                    script: Some(5)
                }),
                |s| {
                    assert_eq!(s, 5);
                    1
                }
            ),
            TalkOutcome::Scripted {
                allowed: true,
                script: 5
            }
        );
        assert_eq!(
            talk(
                Some(TalkOption {
                    status: 7,
                    script: Some(5)
                }),
                |_| 0
            ),
            TalkOutcome::Scripted {
                allowed: false,
                script: 5
            }
        );
    }
}
