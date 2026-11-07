//! The OS layer's pointer rules on the Bevy window: the warp hold after the
//! game moves the pointer itself, and the full-screen confinement.
//!
//! The rules are [`crate::window::WarpHold`] and
//! [`crate::window::FocusConfinement`]; this module applies them to the
//! primary window through winit's pointer position and grab mode.
//!
//! - **Warp hold.** When Cursor Look lets go, the pointer is put back where
//!   it was with the original's hold: for the next four pointer moves a
//!   move that lands 41 px or more from the target (on either axis) puts it
//!   back there, and a move that lands nearer ends the hold. That is how the
//!   original keeps moves the platform still delivers from before the warp
//!   from dragging the pointer away. Wayland moves the pointer only while it
//!   is locked, so there the warp is the lock's position hint and no hold is
//!   kept.
//! - **Full-screen confinement.** Once a capture has ended (a button let go
//!   in the window), a focused full-screen window keeps the pointer inside
//!   it until the focus is lost (`CursorGrabMode::Confined`); Cursor Look's
//!   lock takes precedence while it is held.
//!
//! Rules: `docs/PLAYER.md` (Cursor Look), `docs/PERFORMANCE.md` (the OS
//! window).

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorMoved, CursorOptions, PrimaryWindow, WindowMode};

use crate::window::{FocusConfinement, WarpHold};

/// The warp hold in force on the primary window (client pixels, physical).
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct PointerWarp(pub WarpHold);

impl PointerWarp {
    /// Warps the window's pointer to `to` (logical pixels) with the hold,
    /// as the original's warp does: the hold starts only for a target
    /// inside the client area.
    pub fn warp(&mut self, window: &mut Window, to: Vec2) {
        let scale = window.scale_factor();
        let target = (to * scale).round().as_ivec2();
        let client = IVec2::new(
            window.physical_width() as i32,
            window.physical_height() as i32,
        );
        let at = self.0.warp(target, client);
        window.set_physical_cursor_position(Some(at.as_dvec2()));
    }
}

/// The full-screen confinement's state.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct PointerConfinement(pub FocusConfinement);

/// The pointer's grab for this frame: Cursor Look's lock while it holds,
/// else confined when the full-screen rule asks for it, else free.
pub fn pointer_grab(look_locked: bool, confines: bool) -> CursorGrabMode {
    if look_locked {
        CursorGrabMode::Locked
    } else if confines {
        CursorGrabMode::Confined
    } else {
        CursorGrabMode::None
    }
}

/// Whether the session is a Wayland one, where the pointer can be moved
/// only while it is locked.
pub fn wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub struct OsPointerPlugin;

impl Plugin for OsPointerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PointerWarp>()
            .init_resource::<PointerConfinement>()
            .add_systems(PreUpdate, settle_warp.after(bevy::input::InputSystems))
            .add_systems(PostUpdate, confine_full_screen);
    }
}

/// The window procedure's pointer-move rule while a warp holds: each move
/// is measured against the target and either ends the hold or puts the
/// pointer back (which also makes the position the game reads this frame
/// the target's, as the original's pointer read returns it while held).
fn settle_warp(
    mut moves: MessageReader<CursorMoved>,
    mut warp: ResMut<PointerWarp>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
) {
    if warp.0.remaining < 1 {
        moves.clear();
        return;
    }
    let Ok((entity, mut window)) = windows.single_mut() else {
        moves.clear();
        return;
    };
    let scale = window.scale_factor();
    // The original answers each move message at once; Exarust sees the
    // frame's moves together, so a put-back stands unless a later move
    // settles the hold.
    let mut back = None;
    for event in moves.read().filter(|m| m.window == entity) {
        if warp.0.remaining < 1 {
            break;
        }
        let at = (event.position * scale).round().as_ivec2();
        back = warp.0.moved(at);
    }
    if let Some(target) = back.filter(|_| !wayland()) {
        window.set_physical_cursor_position(Some(target.as_dvec2()));
    }
}

/// The full-screen confinement: armed by a button let go in the focused
/// window, dropped with the focus, applied as winit's confined grab unless
/// Cursor Look holds the lock.
fn confine_full_screen(
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut state: ResMut<PointerConfinement>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
) {
    let Ok((window, mut options)) = windows.single_mut() else {
        return;
    };
    state.0.focus(window.focused);
    if buttons.is_some_and(|b| b.get_just_released().next().is_some()) {
        state.0.capture_ended(window.focused);
    }
    let full_screen = !matches!(window.mode, WindowMode::Windowed);
    let confines = state.0.confines(full_screen, window.focused);
    let grab = pointer_grab(options.grab_mode == CursorGrabMode::Locked, confines);
    if options.grab_mode != grab {
        options.grab_mode = grab;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_look_lock_wins_over_the_confinement() {
        assert_eq!(pointer_grab(true, true), CursorGrabMode::Locked);
        assert_eq!(pointer_grab(true, false), CursorGrabMode::Locked);
        assert_eq!(pointer_grab(false, true), CursorGrabMode::Confined);
        assert_eq!(pointer_grab(false, false), CursorGrabMode::None);
    }

    #[test]
    fn a_warp_inside_the_window_holds_and_one_outside_does_not() {
        let mut window = Window {
            resolution: bevy::window::WindowResolution::new(800, 600),
            ..default()
        };
        let mut warp = PointerWarp::default();
        warp.warp(&mut window, Vec2::new(100.0, 200.0));
        assert_eq!(warp.0.remaining, crate::window::WARP_HOLD_MOVES);
        assert_eq!(warp.0.target, IVec2::new(100, 200));
        assert_eq!(
            window.physical_cursor_position(),
            Some(Vec2::new(100.0, 200.0))
        );
        let mut outside = PointerWarp::default();
        outside.warp(&mut window, Vec2::new(900.0, 10.0));
        assert_eq!(outside.0.remaining, 0);
    }

    #[test]
    fn stale_moves_are_put_back_until_one_lands_near() {
        let mut app = App::new();
        app.add_message::<CursorMoved>()
            .init_resource::<PointerWarp>()
            .add_systems(Update, settle_warp);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(800, 600),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.world_mut().resource_mut::<PointerWarp>().0 = WarpHold {
            remaining: 4,
            target: IVec2::new(400, 300),
        };
        let moved = |app: &mut App, x: f32, y: f32| {
            app.world_mut().write_message(CursorMoved {
                window,
                position: Vec2::new(x, y),
                delta: None,
            });
            app.update();
            app.world().resource::<PointerWarp>().0.remaining
        };
        // A stale move far away is put back and uses up one move.
        assert_eq!(moved(&mut app, 10.0, 10.0), 3);
        // One 40 px off on both axes settles it.
        assert_eq!(moved(&mut app, 360.0, 260.0), 0);
        // With the hold off, moves are free.
        assert_eq!(moved(&mut app, 10.0, 10.0), 0);
    }
}
