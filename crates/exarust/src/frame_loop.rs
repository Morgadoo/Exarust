//! The frame loop's pacing and window controls.
//!
//! Applies the original's frame-rate cap and unfocused hold
//! ([`exa_game::frame`]) to the running winit event loop, empties the held
//! keys and buttons when the focus changes, and binds Alt+Enter to the
//! windowed/full-screen toggle ([`exa_game::window`]), and minimises a
//! full-screen window that loses the focus. The cap is off unless
//! the command line (`--max-fps`) or the settings file (`FPSLimit`) gives
//! one, so no frame is slowed by default: the swap chain's vertical sync and
//! Bevy's schedules already run the loop.
//!
//! The unfocused hold is [`bevy::winit::WinitSettings`], set at start-up in
//! the launcher. The settings' full screen and vertical sync are applied by
//! `exa_game::hw_config`.

use std::time::Instant;

use bevy::prelude::*;
use bevy::window::{Monitor, PresentMode, PrimaryMonitor, PrimaryWindow, WindowMode};
use exa_game::frame::FramePacer;
use exa_game::window::{
    MINIMIZE_FULL_SCREEN_ON_DEACTIVATE, display_refresh_hz, minimizes_on_deactivate, toggled_style,
    window_mode,
};

use crate::Options;

pub struct FrameLoopPlugin;

impl Plugin for FrameLoopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FramePacing>()
            .init_resource::<PacerClock>()
            .init_resource::<WindowedFrame>()
            .add_systems(Startup, take_fps_cap)
            .add_systems(First, pace_frame)
            .add_systems(
                PreUpdate,
                (
                    toggle_windowed,
                    clear_keys_on_focus_change.after(bevy::input::InputSystems),
                    minimize_on_deactivate,
                ),
            );
    }
}

/// The frame-rate cap in force: positive is a fixed rate, a negative value
/// automatic, zero off (the default). The original's cap only ever holds
/// the loop back.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FramePacing {
    pub fps_cap: i32,
}

/// The millisecond clock and the last frame start the pacer measures from.
#[derive(Resource)]
struct PacerClock {
    start: Instant,
    last_ms: u32,
}

impl Default for PacerClock {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            last_ms: 0,
        }
    }
}

/// The window frame Alt+Enter returns to.
#[derive(Resource, Default)]
struct WindowedFrame {
    size: Option<(f32, f32)>,
    position: Option<IVec2>,
}

fn take_fps_cap(options: Option<Res<Options>>, mut pacing: ResMut<FramePacing>) {
    if let Some(options) = options {
        pacing.fps_cap = options.max_fps.unwrap_or(0);
    }
}

/// Holds the frame to the configured rate, measuring from the previous
/// frame's start as the original does. With no cap nothing happens.
fn pace_frame(
    pacing: Res<FramePacing>,
    mut clock: ResMut<PacerClock>,
    windows: Query<&Window, With<PrimaryWindow>>,
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
) {
    if pacing.fps_cap == 0 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let refresh_hz = display_refresh_hz(
        monitors
            .single()
            .ok()
            .and_then(|monitor| monitor.refresh_rate_millihertz),
    );
    let vsync = !matches!(
        window.present_mode,
        PresentMode::AutoNoVsync | PresentMode::Immediate
    );
    let pacer = FramePacer {
        fps_cap: pacing.fps_cap,
        focused: window.focused,
        vsync,
        refresh_hz,
    };
    let now = clock.start.elapsed().as_millis() as u32;
    let wait = pacer.wait_ms(now, clock.last_ms);
    if wait > 0 {
        exa_game::frame::sleep_yield(wait);
    }
    clock.last_ms = clock.start.elapsed().as_millis() as u32;
}

/// Releases the held keys and mouse buttons when the window gains or loses
/// the focus, as the original empties its 256-entry key table on both
/// focus messages. Bevy already releases the keys on a focus loss; the
/// mouse buttons and the gain are Exarust's addition. Runs after Bevy's
/// input systems so the releases are this frame's.
fn clear_keys_on_focus_change(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut keys: Option<ResMut<ButtonInput<KeyCode>>>,
    mut buttons: Option<ResMut<ButtonInput<MouseButton>>>,
    mut focused: Local<Option<bool>>,
) {
    let Ok(window) = windows.single() else { return };
    let was = focused.replace(window.focused);
    if was.is_none_or(|was| was == window.focused) {
        return;
    }
    if let Some(keys) = keys.as_mut()
        && keys.get_pressed().next().is_some()
    {
        keys.release_all();
    }
    if let Some(buttons) = buttons.as_mut()
        && buttons.get_pressed().next().is_some()
    {
        buttons.release_all();
    }
}

/// A full-screen window minimises when it loses the focus to another
/// application, as the original's does on its activation message (its
/// switch for this is on in the shipped game). Unattended runs (a
/// screenshot, fixed frames) keep their window: Exarust's addition, so a
/// scripted run in the background still draws.
fn minimize_on_deactivate(
    options: Option<Res<Options>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut focused: Local<Option<bool>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let was = focused.replace(window.focused);
    if was != Some(true) || window.focused {
        return;
    }
    if options
        .as_ref()
        .is_some_and(|o| o.screenshot.is_some() || o.fixed_frames)
    {
        return;
    }
    let full_screen = !matches!(window.mode, WindowMode::Windowed);
    if minimizes_on_deactivate(MINIMIZE_FULL_SCREEN_ON_DEACTIVATE, full_screen, false) {
        window.set_minimized(true);
    }
}

/// Alt+Enter switches between a framed window and full screen over the
/// desktop, restoring the size and position the framed window had, as the
/// original's system-key handler does (it ignores the key while its window
/// is fixed-size). The original sees the key's auto-repeat as well; Exarust
/// toggles once a press.
fn toggle_windowed(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut saved: ResMut<WindowedFrame>,
) {
    let Some(keys) = keys else { return };
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if !alt || !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let style = toggled_style(window.mode);
    match style {
        exa_game::window::WindowStyle::Windowed => {
            window.mode = window_mode(style);
            if let Some((width, height)) = saved.size {
                window.resolution.set(width, height);
            }
            if let Some(position) = saved.position {
                window.position = bevy::window::WindowPosition::At(position);
            }
        }
        _ => {
            saved.size = Some((window.resolution.width(), window.resolution.height()));
            saved.position = match window.position {
                bevy::window::WindowPosition::At(p) => Some(p),
                _ => None,
            };
            window.mode = window_mode(style);
        }
    }
}

/// The window mode a launch full-screen flag asks for: a borderless window
/// over the desktop, as the original's full screen is while its
/// `ScreenWidth`/`ScreenHeight` settings are unset. With a settings file,
/// `exa_game::hw_config` replaces it with the settings' choice.
pub fn launch_window_mode(fullscreen: bool) -> WindowMode {
    if fullscreen {
        window_mode(exa_game::window::WindowStyle::Borderless)
    } else {
        WindowMode::Windowed
    }
}
