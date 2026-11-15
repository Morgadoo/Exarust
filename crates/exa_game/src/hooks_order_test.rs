//! The original's hook lists against Exarust's schedule
//! (`docs/re/hook-lists.md`): each list's work runs where the original calls
//! the list, in its priority order. (Included from hooks.rs.)

use bevy::prelude::*;
use exa_physics::schedule_order::{conflicting, main_position, runs_before, systems_named};

/// The plugins whose systems do the scene frame list's work.
fn frame_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::transform::TransformPlugin,
        exa_physics::PhysicsPlugin,
        crate::player_light::PlayerLightPlugin,
        exa_world::room_view::RoomViewPlugin,
        exa_world::scene_live::SceneLivePlugin,
    ));
    app
}

/// The scene's frame list, which the original calls at the start of the
/// scene's frame (before the frame sorts the nodes into its sectors), in
/// its priority order: the simulation's hook moves every body node between
/// its last two ticks, then the light over the player's head, then the
/// rooms the camera sees; the scene's own frame comes after them all.
///
/// In Exarust the simulation's frame part (the step packets applied, the
/// render fraction set, the characters drawn) runs in the fixed main loop,
/// before `Update`; the light is placed in `Update` from the head drawn at
/// that fraction; the bodies are drawn before transforms propagate; the
/// rooms are tracked after they propagate (with this frame's camera) and
/// the live scene's frame after the rooms.
#[test]
fn the_scene_frame_list_runs_in_its_priority_order() {
    let mut app = frame_app();
    let fixed = main_position(&app, RunFixedMainLoop).unwrap();
    let update = main_position(&app, Update).unwrap();
    let post = main_position(&app, PostUpdate).unwrap();
    assert!(fixed < update && update < post);

    // Simulation (priority 6): the characters drawn after the fixed loop.
    assert_eq!(
        systems_named(&mut app, RunFixedMainLoop, "show_characters").len(),
        1
    );
    // Its body nodes: drawn before the transforms propagate.
    assert!(runs_before(
        &mut app,
        PostUpdate,
        "body::draw_bodies",
        "propagate_parent_transforms"
    ));
    // The player light (priority 7): in the frame's Update, after the
    // fixed loop set the fraction it draws the head at.
    assert_eq!(
        systems_named(&mut app, Update, "player_light::place_player_light").len(),
        1
    );
    // The rooms (priority 10): after this frame's camera propagated, before
    // the scene's frame.
    assert!(runs_before(
        &mut app,
        PostUpdate,
        "propagate_parent_transforms",
        "room_view::track_rooms"
    ));
    assert!(runs_before(
        &mut app,
        PostUpdate,
        "room_view::track_rooms",
        "scene_live::run_scene"
    ));
    assert!(!runs_before(
        &mut app,
        PostUpdate,
        "scene_live::run_scene",
        "room_view::track_rooms"
    ));
}

/// The game task's frame list, which the original calls once a frame in
/// its priority order: the session's frame, then the player controls'
/// frame (the held item and the use list), then the scene task (whose
/// scene frame list moves the camera), then the spirit screen. Exarust's
/// pieces run in `Update`; where the schedule leaves two of them unordered
/// their data does not conflict, so either order computes the same, and
/// the held item reads the camera's propagated transform, which in `Update`
/// is still where the previous frame left it: where the original's
/// controls, called before the camera's hook, find it.
#[test]
fn the_game_task_list_keeps_its_order_by_schedule_or_by_data() {
    let mut app = frame_app();
    app.add_plugins((
        crate::player::PlayerPlugin,
        crate::hand::HandPlugin,
        crate::menu_screen::MenuScreenPlugin,
        crate::flow::FlowPlugin,
        crate::mind::runtime::MindRuntimePlugin,
    ));
    for name in [
        "flow::tick",
        "hand::hold_hand",
        "player::chase_camera",
        "mind::runtime::step_spirit_screen",
        "flow::run_transition",
    ] {
        assert_eq!(systems_named(&mut app, Update, name).len(), 1, "{name}");
    }
    assert!(runs_before(
        &mut app,
        Update,
        "flow::tick",
        "flow::run_transition"
    ));
    // The held item comes after the camera's own tick in Exarust, but it
    // reads the camera's propagated transform (the previous frame's).
    assert!(runs_before(
        &mut app,
        Update,
        "player::chase_camera",
        "hand::hold_hand"
    ));
    // The rest are unordered, and none of them conflicts with another.
    for (a, b) in [
        ("flow::tick", "hand::hold_hand"),
        ("flow::tick", "player::chase_camera"),
        ("flow::tick", "player_light::place_player_light"),
        ("flow::tick", "mind::runtime::step_spirit_screen"),
        ("hand::hold_hand", "player_light::place_player_light"),
        ("hand::hold_hand", "mind::runtime::step_spirit_screen"),
        ("player::chase_camera", "player_light::place_player_light"),
        ("player::chase_camera", "mind::runtime::step_spirit_screen"),
    ] {
        assert!(!conflicting(&mut app, Update, a, b), "{a} and {b}");
    }
}
