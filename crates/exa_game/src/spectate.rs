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
//! - the chase camera looks at the followed body's pelvis; a free camera
//!   runs the controller's free branch from the point it last looked at:
//!   W/A/S/D pan that point along the view, the terrain lifts it toward a
//!   metre over the ground, and its height stays within the preset's
//!   0–2 m (`crate::nav_camera::FreeFocus`).
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
    /// An arena match's entry started the session (no entrant was the
    /// player's): the hub's arrival ends it.
    pub arena: bool,
    /// The camera placement the arena's start asks for: the eye and the
    /// point it looks at (m). The chase camera places itself there once,
    /// free, and takes it.
    pub pose: Option<(Vec3, Vec3)>,
    /// The point whose room the room walk is forced to (m): the arena's
    /// spawn spot, as the original's match start stores the room there.
    /// None outside an arena match, where the walk then keeps its rooms.
    pub forced_room_at: Option<Vec3>,
}

/// Where the original's spectated match start puts the camera: the arena's
/// spawn spot raised by this, centimetres, looking at the spot.
pub const SPECTATE_CAMERA_OFFSET_CM: [f32; 3] = [700.0, 1180.0, 1100.0];

impl SpectateSession {
    /// The session's start: the flag set, the camera following nothing.
    pub fn start(&mut self) {
        self.active = true;
        self.follow = None;
        self.last_press = None;
    }

    /// An arena match's start with no entrant of the player's: the
    /// session starts with the camera looking at the spawn spot `spot`
    /// (metres).
    pub fn start_arena(&mut self, spot: Vec3) {
        self.start();
        self.arena = true;
        self.focus = Some(spot);
        self.pose = Some((
            spot + Vec3::from_array(SPECTATE_CAMERA_OFFSET_CM) / 100.0,
            spot,
        ));
        self.forced_room_at = Some(spot);
    }

    /// The session's end: the flag cleared, the camera's follow cleared.
    pub fn end(&mut self) {
        self.active = false;
        self.follow = None;
        self.last_press = None;
        self.arena = false;
        self.pose = None;
        self.forced_room_at = None;
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
    control: Option<ResMut<crate::player_controls::PlayerControl>>,
) {
    if loading.is_some() {
        return;
    }
    session.start();
    if let Some(mut control) = control {
        crate::controls_live::switch_controls(&mut control, false, "spectate starts");
    }
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
    if follow.is_some() {
        session.pose = None;
    }
    session.follow = follow;
}

/// The end of an arena match's session: the mode changes back to
/// Character mode when the hub's arrival starts play again (the original's
/// mode end hook runs as the next mode starts), which is when the manager
/// stands in the hub again ([`crate::save_live::SavedPlayer`] has made him
/// from his record and is dropped).
fn end_arena_session(
    mut commands: Commands,
    mut session: ResMut<SpectateSession>,
    scene: Option<Res<crate::arena_live::ArenaScene>>,
    players: Query<(), With<crate::player::Player>>,
    control: Option<ResMut<crate::player_controls::PlayerControl>>,
) {
    if !session.arena || scene.is_some() || players.is_empty() {
        return;
    }
    session.end();
    // The Spectate mode's end turns the controls off, then the Character
    // mode's start turns them on.
    if let Some(mut control) = control {
        crate::controls_live::switch_controls(&mut control, true, "spectate ends");
        crate::controls_live::switch_controls(&mut control, false, "start of play");
    }
    commands.remove_resource::<crate::save_live::SavedPlayer>();
    info!("spectate: session ended; the manager is back in Character mode");
}

/// The hip of the body the spectating camera follows, drawn `alpha` of the
/// way into the tick (the original's character point, which its follow
/// placement and wall probe read); `None` for a free camera, or once the
/// followed body is gone.
pub(crate) fn followed_hip(
    session: &SpectateSession,
    bodies: &Query<&Ragdoll>,
    alpha: f32,
) -> Option<Vec3> {
    session
        .follow
        .and_then(|e| bodies.get(e).ok())
        .and_then(|b| b.parts.iter().find(|p| p.parent.is_none()))
        .map(|pelvis| Vec3::from(pelvis.drawn(alpha).translation))
}

/// The room walk follows the session: while it runs the view follows no
/// player, so the walk takes its forced-room branch with the arena's spawn
/// spot (the original's walk tests the scene's control mode each frame and
/// its match start stores the room at the spot).
///
/// The point holds only in the match's own level: the original stores the
/// room's index for the arena, and its hub load clears it, so the hub the
/// return loads (still in Spectate until the manager stands) keeps its rooms.
fn force_the_room_walk(
    session: Res<SpectateSession>,
    scene: Option<Res<crate::arena_live::ArenaScene>>,
    mut forced: ResMut<exa_world::room_view::ForcedRoom>,
) {
    let want = exa_world::room_view::ForcedRoom {
        active: session.active,
        at: session
            .forced_room_at
            .filter(|_| scene.is_some())
            .map(|p| Vec2::new(p.x, p.z)),
    };
    if *forced != want {
        info!(
            "room walk: {} (forced to the room at {:?})",
            if want.active {
                "the view follows no player"
            } else {
                "the view follows the player"
            },
            want.at
        );
        *forced = want;
    }
}

pub struct SpectatePlugin;

impl Plugin for SpectatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpectateSession>().add_systems(
            Update,
            (
                start_on_load.run_if(resource_exists::<SpectateOnLoad>),
                spectate_clicks.run_if(|s: Res<SpectateSession>| s.active),
                end_arena_session.run_if(|s: Res<SpectateSession>| s.active),
                force_the_room_walk.run_if(resource_exists::<exa_world::room_view::ForcedRoom>),
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

    /// An arena match's session looks at the spawn spot and its end
    /// clears the arena mark.
    #[test]
    fn an_arena_session_looks_at_the_spawn_spot() {
        let mut s = SpectateSession::default();
        s.start_arena(Vec3::new(1.0, 2.0, 3.0));
        assert!(s.active && s.arena && s.follow.is_none());
        assert_eq!(s.focus, Some(Vec3::new(1.0, 2.0, 3.0)));
        // The camera stands 7, 11.8 and 11 m off the spot, looking at it.
        let (eye, at) = s.pose.unwrap();
        assert!(eye.abs_diff_eq(Vec3::new(8.0, 13.8, 14.0), 1e-5), "{eye}");
        assert_eq!(at, Vec3::new(1.0, 2.0, 3.0));
        // The room walk is forced to the spot's room.
        assert_eq!(s.forced_room_at, Some(Vec3::new(1.0, 2.0, 3.0)));
        s.end();
        assert!(!s.active && !s.arena && s.pose.is_none());
        assert_eq!(s.forced_room_at, None);
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
