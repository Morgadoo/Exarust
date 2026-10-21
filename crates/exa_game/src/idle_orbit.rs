//! The idle camera: left alone for ten seconds, the chase camera starts to
//! turn about the player. Rules: `docs/PLAYER.md` (Camera, The idle turn).
//!
//! Each frame of play the player's controls test whether the player is
//! idle: the pointer within 2 pixels of where it was last seen on both
//! axes, the player's condition word plain (0) and no foot stepping, the
//! left button holding nothing, and no mouse button held. A dead player is
//! idle whatever the pointer does. When it is not idle the pointer's place
//! and the stamp are taken now. When it is, once the game clock is past the
//! stamp plus 10 000 ms, the camera's target yaw turns by 0.0001 rad for
//! each millisecond since the last frame (0.1 rad a second; in double
//! precision, then single), less a turn (2π) once past it. While it turns
//! the behaviour stops reading the cursor (the head and the heading keep
//! the cursor's last direction), until the player is busy again.
//!
//! Not in Exarust: the screens that block the controls (the death screen,
//! the in-game menu), which turn the camera the same way without the idle
//! test; an open window that stops the turn; the dialogue and power bar
//! that make the player busy.
//!
//! Evidence: the controls' frame and its blocked branch, read in the
//! instructions (`context/re/player-control-hand.md`, The idle turn).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_combat::Condition;
use exa_physics::body::Ragdoll;
use exa_ui::ScriptedPointer;

use crate::hand::{Hand, Holding};
use crate::player::{ChaseCamera, PlayerBody};

/// Game time (ms) the player must be idle before the camera turns.
pub const IDLE_MS: i64 = 10_000;
/// The turn (rad) per millisecond of game time.
pub const TURN_PER_MS: f64 = 0.0001;
/// The pointer may wander this many pixels (exclusive) on each axis.
const STILL_PX: i64 = 3;

/// The controls' idle watch between frames.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleWatch {
    /// When the player was last seen busy (game ms).
    pub stamp: u32,
    /// The last frame's game clock (ms).
    pub last: u32,
    /// Where the pointer was last seen (window pixels).
    pub pointer: IVec2,
    /// The behaviour reads the cursor: off while the camera turns.
    pub follows_cursor: bool,
}

impl Default for IdleWatch {
    fn default() -> Self {
        Self {
            stamp: 0,
            last: 0,
            pointer: IVec2::ZERO,
            follows_cursor: true,
        }
    }
}

/// What the idle test reads of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The game clock (ms).
    pub now: u32,
    /// The pointer (window pixels); `None` keeps the last.
    pub pointer: Option<IVec2>,
    /// The player's condition word.
    pub condition: Condition,
    /// A foot is stepping.
    pub stepping: bool,
    /// The left button holds something.
    pub holding: bool,
    /// A mouse button is held.
    pub buttons: bool,
}

/// Whether the pointer stayed within 2 pixels of `was` on both axes.
pub fn pointer_still(was: IVec2, now: IVec2) -> bool {
    let dx = (i64::from(was.x) - i64::from(now.x)).abs();
    let dy = (i64::from(was.y) - i64::from(now.y)).abs();
    dx < STILL_PX && dy < STILL_PX
}

/// The target yaw after a frame's turn: 0.0001 rad a millisecond since
/// `last` (double precision, then single), less 2π once past it.
pub fn turned(yaw: f32, now: u32, last: u32) -> f32 {
    let ms = i64::from(now) - i64::from(last);
    let y = (ms as f64 * TURN_PER_MS + f64::from(yaw)) as f32;
    if y > std::f32::consts::TAU {
        y - std::f32::consts::TAU
    } else {
        y
    }
}

impl IdleWatch {
    /// One frame of the watch: the camera's yaw after it, from `yaw`.
    pub fn frame(&mut self, f: Frame, yaw: f32) -> f32 {
        let still = if f.condition.is_dead() {
            true
        } else {
            f.pointer.is_none_or(|p| pointer_still(self.pointer, p))
                && f.condition == Condition::NORMAL
                && !f.stepping
                && !f.holding
        };
        if !f.buttons && still {
            let mut yaw = yaw;
            if i64::from(self.stamp) + IDLE_MS < i64::from(f.now) {
                self.follows_cursor = false;
                yaw = turned(yaw, f.now, self.last);
            }
            self.last = f.now;
            yaw
        } else {
            if let Some(p) = f.pointer {
                self.pointer = p;
            }
            self.stamp = f.now;
            self.follows_cursor = true;
            yaw
        }
    }
}

pub struct IdleOrbitPlugin;

impl Plugin for IdleOrbitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IdleWatch>().add_systems(
            Update,
            watch_idle
                .after(crate::hand::HandSystems)
                .before(crate::player::chase_camera),
        );
    }
}

/// The frame's watch for the player, turning the chase camera.
#[allow(clippy::too_many_arguments)]
fn watch_idle(
    time: Res<Time<Virtual>>,
    buttons: Res<ButtonInput<MouseButton>>,
    scripted: Res<ScriptedPointer>,
    hand: Option<Res<Hand>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<(Option<&Condition>, Option<&Ragdoll>), With<PlayerBody>>,
    mut cameras: Query<&mut ChaseCamera>,
    mut watch: ResMut<IdleWatch>,
) {
    let Some((condition, body)) = players.iter().next() else {
        return;
    };
    let pointer = windows.single().ok().and_then(|w| {
        scripted
            .position(w)
            .map(|p| (p * w.scale_factor()).as_ivec2())
    });
    let frame = Frame {
        now: time.elapsed().as_millis() as u32,
        pointer,
        condition: condition.copied().unwrap_or_default(),
        stepping: body
            .and_then(|b| b.drive.as_ref())
            .is_some_and(|d| d.stepping()),
        holding: hand.is_some_and(|h| h.holding != Holding::Nothing),
        buttons: buttons.get_pressed().next().is_some(),
    };
    for mut cam in &mut cameras {
        let yaw = watch.frame(frame, cam.yaw);
        if yaw != cam.yaw {
            cam.yaw = yaw;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle(now: u32) -> Frame {
        Frame {
            now,
            pointer: Some(IVec2::new(100, 100)),
            condition: Condition::NORMAL,
            stepping: false,
            holding: false,
            buttons: false,
        }
    }

    /// Two pixels is still, three is not, on either axis.
    #[test]
    fn the_pointer_may_wander_two_pixels() {
        let at = IVec2::new(100, 100);
        assert!(pointer_still(at, IVec2::new(102, 98)));
        assert!(!pointer_still(at, IVec2::new(103, 100)));
        assert!(!pointer_still(at, IVec2::new(100, 97)));
    }

    /// 0.1 rad a second of game time, wrapped past a turn.
    #[test]
    fn the_turn_is_a_tenth_of_a_radian_a_second() {
        assert!((turned(1.0, 2000, 1000) - 1.1).abs() < 1e-6);
        let past = turned(6.25, 1500, 1000);
        assert!(
            (past - (6.30 - std::f32::consts::TAU)).abs() < 1e-5,
            "{past}"
        );
        assert_eq!(turned(6.2, 1000, 1000), 6.2);
    }

    /// The camera waits ten seconds of idleness (strictly past the stamp
    /// plus 10 000 ms), then turns by the frame's time; moving the pointer
    /// three pixels starts the wait again.
    #[test]
    fn ten_idle_seconds_turn_the_camera() {
        let mut w = IdleWatch::default();
        // A busy frame stamps the clock and the pointer.
        let busy = Frame {
            pointer: Some(IVec2::new(100, 100)),
            buttons: true,
            ..idle(1000)
        };
        assert_eq!(w.frame(busy, 0.0), 0.0);
        assert_eq!(w.stamp, 1000);
        assert_eq!(w.frame(idle(11_000), 0.0), 0.0);
        assert!(w.follows_cursor);
        let yaw = w.frame(idle(11_001), 0.0);
        assert!(!w.follows_cursor);
        assert!((yaw - 0.0001).abs() < 1e-9, "{yaw}");
        let yaw = w.frame(idle(12_001), yaw);
        assert!((yaw - 0.1001).abs() < 1e-6, "{yaw}");
        let moved = Frame {
            pointer: Some(IVec2::new(103, 100)),
            ..idle(12_002)
        };
        assert_eq!(w.frame(moved, yaw), yaw);
        assert_eq!((w.stamp, w.pointer), (12_002, IVec2::new(103, 100)));
        assert!(w.follows_cursor);
        assert_eq!(w.frame(idle(13_000), yaw), yaw);
    }

    /// Walking (word 1), a stepping foot, a hold or a button keep the camera
    /// still; a dead player turns it even with the pointer moving.
    #[test]
    fn a_busy_player_keeps_the_camera_still_and_a_dead_one_does_not() {
        for busy in [
            Frame {
                condition: Condition::WALKING,
                ..idle(20_000)
            },
            Frame {
                stepping: true,
                ..idle(20_000)
            },
            Frame {
                holding: true,
                ..idle(20_000)
            },
            Frame {
                buttons: true,
                ..idle(20_000)
            },
        ] {
            let mut w = IdleWatch {
                pointer: IVec2::new(100, 100),
                ..IdleWatch::default()
            };
            assert_eq!(w.frame(busy, 0.5), 0.5);
            assert_eq!(w.stamp, 20_000);
        }
        let mut w = IdleWatch::default();
        let dead = Frame {
            condition: Condition::DEAD,
            pointer: Some(IVec2::new(900, 900)),
            ..idle(20_000)
        };
        let yaw = w.frame(dead, 0.5);
        assert!((yaw - 2.5).abs() < 1e-6, "{yaw}");
    }
}
