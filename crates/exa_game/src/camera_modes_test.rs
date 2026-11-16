//! The game modes live: each mode's start enters its mode through the
//! scene's mode word ([`crate::player_controls::GameModes`]) and the chase
//! camera takes that mode's record, with the real player, spectate and
//! controls plugins in a headless app.

use bevy::input::InputPlugin;
use bevy::prelude::*;

use crate::player::{ChaseCamera, Player};
use crate::player_controls::{GameMode, GameModes, PlayerControl};
use crate::spectate::{SpectateOnLoad, SpectateSession};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        InputPlugin,
        crate::player::PlayerPlugin,
        crate::spectate::SpectatePlugin,
        crate::controls_live::ControlsLivePlugin,
    ));
    app.insert_resource(exa_world::room_view::ForcedRoom::default())
        .init_resource::<exa_ui::cursor::CursorCapture>()
        .init_resource::<exa_ui::ScriptedPointer>()
        .add_message::<exa_combat::impact::CameraHit>();
    // The plugins' systems whose resources this app does not build (the
    // physics, the assets) are skipped; any other error still fails.
    app.set_error_handler(|error, ctx| {
        if error
            .downcast_ref::<bevy::ecs::system::SystemParamValidationError>()
            .is_none()
        {
            bevy::ecs::error::panic(error, ctx);
        }
    });
    // A frame is 20 ms, so each frame runs a camera tick.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(20),
    ));
    app
}

/// Holds W for `frames` frames and answers how far the free camera's point
/// moved (m).
fn pan_forward(app: &mut App, camera: Entity, frames: usize) -> f32 {
    let from = cam(app, camera).free_focus(1.0).unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    for _ in 0..frames {
        app.update();
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyW);
    cam(app, camera).free_focus(1.0).unwrap().distance(from)
}

fn camera(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            ChaseCamera::default(),
            Transform::from_xyz(0.0, 4.0, 4.0),
            Projection::Perspective(PerspectiveProjection::default()),
        ))
        .id()
}

fn cam(app: &App, camera: Entity) -> &ChaseCamera {
    app.world().get::<ChaseCamera>(camera).unwrap()
}

/// The start of play enters the Character mode: a camera built from the
/// Character record takes that as its construction (its zoom kept). A
/// spectate start then leaves Character (its end of play) and enters
/// Spectate: the camera takes the Spectate record with the scan bit and
/// its zoom starts again from the record's 1300; the room walk is forced.
/// The player taken away in Spectate does not end play a second time, and
/// the controls stay on.
#[test]
fn each_mode_start_gives_the_camera_its_record() {
    let mut app = app();
    let camera = camera(&mut app);
    app.update();
    // No mode before the player exists: the camera keeps its record.
    assert_eq!(app.world().resource::<GameModes>().current(), None);
    assert_eq!(cam(&app, camera).record_flags(), 0x0020_0300);
    let player = app.world_mut().spawn((Player, Transform::default())).id();
    app.update();
    let modes = app.world().resource::<GameModes>();
    assert!(modes.is(GameMode::Character));
    assert_eq!(modes.active, 3);
    assert_eq!(cam(&app, camera).mode, Some(GameMode::Character));
    assert_eq!(cam(&app, camera).record_flags(), 0x0020_0300);
    assert!(!app.world().resource::<PlayerControl>().off);
    assert!(
        !app.world()
            .resource::<exa_world::room_view::ForcedRoom>()
            .active
    );
    // The wheel zooms out to the last stop.
    app.world_mut()
        .write_message(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -10.0,
            window: Entity::PLACEHOLDER,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
    app.update();
    assert_eq!(cam(&app, camera).zoom(), 2000.0);
    // --spectate's start.
    app.world_mut().insert_resource(SpectateOnLoad);
    app.update();
    let modes = app.world().resource::<GameModes>();
    assert!(modes.is(GameMode::Spectate));
    assert_eq!(modes.active, 4);
    assert!(app.world().resource::<SpectateSession>().active);
    assert_eq!(cam(&app, camera).mode, Some(GameMode::Spectate));
    assert_eq!(cam(&app, camera).record_flags(), 0x0062_0b01);
    assert_eq!(cam(&app, camera).zoom(), 1300.0);
    // The room walk reads the mode word in the next frame.
    app.update();
    assert!(
        app.world()
            .resource::<exa_world::room_view::ForcedRoom>()
            .active
    );
    // The spectating camera follows nothing: the Spectate record's pan keys
    // move its point; with the Character record (no `0x800`) W does not.
    assert!(cam(&app, camera).is_free());
    assert!(pan_forward(&mut app, camera, 10) > 0.01);
    app.world_mut()
        .get_mut::<ChaseCamera>(camera)
        .unwrap()
        .take_record(GameMode::Character.camera_record(false));
    for _ in 0..200 {
        app.update();
    }
    assert_eq!(pan_forward(&mut app, camera, 10), 0.0);
    // The player taken away while spectating: the mode stays Spectate and
    // the controls stay on.
    app.world_mut().despawn(player);
    app.update();
    app.update();
    assert!(app.world().resource::<GameModes>().is(GameMode::Spectate));
    assert!(!app.world().resource::<PlayerControl>().off);
}

/// The player gone in Character mode is the level's unload: the scene
/// switches to no mode, the controls go off and the room walk is not
/// forced; the next player enters Character again, and a camera that has
/// held another record takes the Character one.
#[test]
fn the_end_of_play_leaves_the_scene_in_no_mode() {
    let mut app = app();
    let camera = camera(&mut app);
    let player = app.world_mut().spawn((Player, Transform::default())).id();
    app.update();
    assert!(app.world().resource::<GameModes>().is(GameMode::Character));
    app.world_mut().despawn(player);
    app.update();
    let modes = app.world().resource::<GameModes>();
    assert_eq!((modes.active, modes.current()), (0, None));
    assert!(app.world().resource::<PlayerControl>().off);
    assert!(
        !app.world()
            .resource::<exa_world::room_view::ForcedRoom>()
            .active
    );
    // The camera keeps its record through the switch to no mode.
    assert_eq!(cam(&app, camera).record_flags(), 0x0020_0300);
    // A camera given the Spectate record by hand takes the Character one
    // at the next start of play.
    app.world_mut()
        .get_mut::<ChaseCamera>(camera)
        .unwrap()
        .take_record(GameMode::Spectate.camera_record(false));
    app.world_mut().spawn((Player, Transform::default()));
    app.update();
    assert!(app.world().resource::<GameModes>().is(GameMode::Character));
    assert_eq!(cam(&app, camera).record_flags(), 0x0020_0300);
}
