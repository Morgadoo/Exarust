//! The spectate session: the controls' Spectate game mode, which the
//! original's arena enters when the round's fighter is an employee of the
//! player's company rather than the player (the next round's and a
//! tournament round's entry, `docs/re/playerctl-map.md`). Its start turns
//! the controls on with the spectate flag set, frees the camera (it follows
//! nothing) and registers the double click's interaction routine as the
//! session's click; its end turns the controls off, clears the flag and the
//! camera's follow, and drops that click.
//!
//! While the session runs:
//! - the controls' frame does nothing (the player is not steered);
//! - a double click picks under the cursor: a character's body makes the
//!   camera follow it, anything else (or nothing) frees the camera, which
//!   then stays where it looks;
//! - the chase camera looks at the followed body's pelvis, else at the
//!   point it last looked at.
//!
//! Exarust's arena always fields the player, so nothing in play enters the
//! session yet; the play flag `--spectate` enters it once the load is over,
//! in any level, to check it.

use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_physics::body::Ragdoll;
use exa_ui::ScriptedPointer;

use crate::hand::{Hit, OtherBodies, PickSources};
use crate::player::{ChaseCamera, PlayerBody};

/// The spectate session's state.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct SpectateSession {
    /// The session runs (the controls' spectate flag).
    pub active: bool,
    /// The body the camera follows (the camera's follow object), none for
    /// a free camera.
    pub follow: Option<Entity>,
    /// Where the camera last looked (m), which a free camera keeps.
    pub focus: Option<Vec3>,
    /// The last left press (window pixels, ms), for the double click.
    last_press: Option<((i32, i32), u64)>,
}

impl SpectateSession {
    /// The session's start: the flag set, the camera following nothing.
    pub fn start(&mut self) {
        self.active = true;
        self.follow = None;
        self.last_press = None;
    }

    /// The session's end: the flag cleared, the camera's follow cleared.
    pub fn end(&mut self) {
        self.active = false;
        self.follow = None;
        self.last_press = None;
    }

    /// Whether the controls are spectating.
    pub fn spectating(session: Option<&Self>) -> bool {
        session.is_some_and(|s| s.active)
    }
}

/// `--spectate`: the session starts once the loading screen is gone.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct SpectateOnLoad;

/// What the session's double click does with the pick under the cursor:
/// a character's body (living or dead) becomes the camera's follow object;
/// anything else, and a miss, frees the camera.
pub fn spectate_follow(hit: Option<Hit>) -> Option<Entity> {
    match hit {
        Some(Hit::Character { body, .. }) => Some(body),
        _ => None,
    }
}

/// Starts the session for `--spectate` once the load is over.
fn start_on_load(
    mut commands: Commands,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut session: ResMut<SpectateSession>,
) {
    if loading.is_some() {
        return;
    }
    session.start();
    commands.remove_resource::<SpectateOnLoad>();
    info!("spectate: session started");
}

/// The session's double click: the pick under the cursor sets or clears
/// the camera's follow object.
#[allow(clippy::too_many_arguments)]
fn spectate_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    scripted: Res<ScriptedPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    sources: PickSources,
    bodies: Query<OtherBodies, Without<PlayerBody>>,
    mut session: ResMut<SpectateSession>,
) {
    if !session.active || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(at) = scripted.position(window) else {
        return;
    };
    let pixel = (at.x.round() as i32, at.y.round() as i32);
    let now = time.elapsed().as_millis() as u64;
    let double = session.last_press.is_some_and(|(was, was_ms)| {
        crate::player_controls::is_double_click(was, was_ms, pixel, now)
    });
    session.last_press = Some((pixel, now));
    if !double {
        return;
    }
    session.last_press = None;
    let ray = crate::hand::cursor_ray(&scripted, &windows, &cameras);
    let picked = crate::hand::pick_with(
        ray,
        &sources.physics,
        &sources.stand_ins,
        &sources.collider_nodes,
        &bodies,
        None,
    );
    let follow = spectate_follow(picked.map(|p| p.hit));
    if follow != session.follow {
        info!("spectate: the camera follows {follow:?}");
    }
    session.follow = follow;
}

/// The chase camera's focus while spectating: the followed body's pelvis
/// drawn `alpha` of the way into the tick, else the point it last looked
/// at, else `fallback`.
pub(crate) fn spectate_focus(
    session: &SpectateSession,
    bodies: &Query<&Ragdoll>,
    alpha: f32,
    fallback: Vec3,
) -> Vec3 {
    session
        .follow
        .and_then(|e| bodies.get(e).ok())
        .and_then(|b| b.parts.iter().find(|p| p.parent.is_none()))
        .map(|pelvis| Vec3::from(pelvis.drawn(alpha).translation))
        .or(session.focus)
        .unwrap_or(fallback)
}

pub struct SpectatePlugin;

impl Plugin for SpectatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpectateSession>().add_systems(
            Update,
            (
                start_on_load.run_if(resource_exists::<SpectateOnLoad>),
                spectate_clicks.run_if(|s: Res<SpectateSession>| s.active),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The start sets the flag with a free camera; the end clears both.
    #[test]
    fn the_session_starts_free_and_ends_clear() {
        let mut s = SpectateSession {
            follow: Some(Entity::from_raw_u32(7).unwrap()),
            ..default()
        };
        s.start();
        assert!(s.active && s.follow.is_none());
        s.follow = Some(Entity::from_raw_u32(9).unwrap());
        s.end();
        assert!(!s.active && s.follow.is_none());
        assert!(!SpectateSession::spectating(Some(&s)));
        assert!(!SpectateSession::spectating(None));
    }

    /// A character (living or dead) is followed; a prop, the level, a
    /// scene node and a miss free the camera.
    #[test]
    fn a_double_click_on_a_character_follows_it() {
        let body = Entity::from_raw_u32(3).unwrap();
        for dead in [false, true] {
            let hit = Hit::Character {
                body,
                part: 2,
                dead,
            };
            assert_eq!(spectate_follow(Some(hit)), Some(body));
        }
        assert_eq!(spectate_follow(Some(Hit::Level)), None);
        assert_eq!(spectate_follow(Some(Hit::Prop { body, part: 0 })), None);
        assert_eq!(spectate_follow(None), None);
    }
}
