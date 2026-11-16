//! The phases of a drawn frame: every system of the game's `Update` and
//! `PostUpdate` sits in one, the phases run in a fixed chain, and two
//! systems of one phase that touch the same data have their own
//! `before`/`after` (OpenSpec `order-the-frame-schedules`).
//!
//! The frame's systems run on the thread pool, so a pair the schedule
//! leaves unordered runs in whichever order the threads give it, which
//! can change from frame to frame. The original runs one routine per
//! frame in a fixed order: the GUI reads the pointer and the keys and
//! drives its windows and the game's screens; the game task's frame list
//! runs the session's frame, the player controls' frame (the held item and
//! the use list), the scene task (whose scene frame list moves the camera,
//! the player light, the room tracker) and the spirit screen; the modal
//! list runs the transition screen; the render frame then draws the scene
//! and the GUI (`docs/re/hook-lists.md`). The phases follow that order;
//! inside a phase, a pair keeps the order the frame's executor tended to
//! give it before (its topological order).

use bevy::prelude::*;

/// The phases of the frame's `Update`, in this order:
///
/// 1. `Input`: the input read beyond Bevy's (the pointer's world ray and
///    look, the pad's cursor, the debug keys and the command line's queued
///    commands, the item catalogue's load).
/// 2. `Gui`: the GUI's pointer, keys and drive: panels, drags and drops,
///    the use cursor, the hotkeys, the game screens and their paint, the
///    settings, the icon renders.
/// 3. `Hub`: the arena hub's windows and forms, then the item card and the
///    parchment drawn over them.
/// 4. `Session`: the session's frame: the flow and its steps, the level
///    changes, the saves and loads, the scripts, the arena's session and
///    match, the command line's scripted actions. The mind's hand-back
///    transition (an exclusive system) runs between `Session` and
///    `Controls`.
/// 5. `Controls`: the player controls' frame part (the player's combat,
///    the blows felt, the powers' bar and kinesis input).
/// 6. `Camera`: the camera's mode records and its chase, the pad's view.
/// 7. `Hand`: the held item and the use list (the hand, the object clicks,
///    the character uses), the HUD's picks, the idle orbit.
/// 8. `Cursor`: the world cursor's pick and the system cursor.
/// 9. `World`: the world's draw preparation: animation, skins and dress,
///    the lights, flames, arcs, cloth and terrain, the effects' draws.
/// 10. `Audio`: the sounds, the music and the reverb.
/// 11. `Draw`: the HUD's draw, the world cursor's, the loading screen and
///     the debug overlays.
/// 12. `Capture`: the screenshots, readbacks, traces and logs, last.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FramePhase {
    Input,
    Gui,
    Hub,
    Session,
    Controls,
    Camera,
    Hand,
    Cursor,
    World,
    Audio,
    Draw,
    Capture,
}

/// The phases of the frame's `PostUpdate`, in this order, placed against
/// Bevy's own sets:
///
/// 1. `Prepare`: before the transforms propagate (and before the camera
///    update, the UI and the asset events): the settings' syncs, the sky
///    and fog, the pointer's warps, the bodies' draw, the lights' states,
///    the fires' hosts, the mind sight, the GUI frames and overlays.
/// 2. `Placed`: after the transforms propagate, before the bounds, the
///    frusta and the visibility: the effects' draws and lights and their
///    sounds, the blood and sprays, the rooms' doors and lids, the
///    physics' static colliders and props, the light ids.
/// 3. `View`: after the visibility: the live scene, the shadow links, the
///    cube shadows, the light lists, the GI, the cut-away.
/// 4. `Trace`: the draw trace, last.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FramePostPhase {
    Prepare,
    Placed,
    View,
    Trace,
}

/// Chains the frame's phases in `Update` and `PostUpdate` and places the
/// `PostUpdate` ones against Bevy's sets. Called once by the game's plugin
/// (calling it again only repeats the same edges).
pub fn configure_frame_phases(app: &mut App) {
    use FramePhase as F;
    use FramePostPhase as P;
    use bevy::asset::AssetEventSystems;
    use bevy::camera::CameraUpdateSystems;
    use bevy::camera::visibility::VisibilitySystems;
    use bevy::light::SimulationLightSystems;
    use bevy::transform::TransformSystems;
    use bevy::ui::UiSystems;
    app.configure_sets(
        Update,
        (
            F::Input,
            F::Gui,
            F::Hub,
            F::Session,
            F::Controls,
            F::Camera,
            F::Hand,
            F::Cursor,
            F::World,
            F::Audio,
            F::Draw,
            F::Capture,
        )
            .chain(),
    );
    // Bevy's Ctrl-C handler sends its exit after the frame's own exits.
    #[cfg(unix)]
    app.configure_sets(
        Update,
        F::Capture.before(bevy::app::TerminalCtrlCHandlerPlugin::exit_on_flag),
    );
    app.configure_sets(
        PostUpdate,
        (
            P::Prepare
                .before(TransformSystems::Propagate)
                .before(CameraUpdateSystems)
                .before(UiSystems::Prepare)
                .before(AssetEventSystems)
                .before(bevy::core_pipeline::core_3d::check_msaa),
            P::Placed
                .after(TransformSystems::Propagate)
                .before(VisibilitySystems::CalculateBounds)
                .before(VisibilitySystems::UpdateFrusta)
                .before(VisibilitySystems::VisibilityPropagate)
                .before(SimulationLightSystems::UpdateBounds)
                .before(SimulationLightSystems::UpdateDirectionalLightCascades),
            P::View,
            P::Trace,
        )
            .chain(),
    );
}
