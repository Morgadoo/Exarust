//! The frame loop's pacing and window controls.
//!
//! Applies the original's frame-rate cap and unfocused hold
//! ([`exa_game::frame`]) to the running winit event loop, and Alt+Enter's
//! windowed/borderless toggle ([`exa_game::window`]). The default cap is
//! off, so no frame is slowed: the swap chain's vertical sync and Bevy's
//! schedules already run the loop, and a frame cap is an explicit launch
//! option.
//!
//! Window focus changes are handled by winit; the unfocused hold is
//! [`bevy::winit::WinitSettings`], set at start-up in the launcher.

use std::time::Instant;

use bevy::prelude::*;
use bevy::window::{Monitor, PresentMode, PrimaryMonitor, PrimaryWindow, WindowMode};
use exa_game::frame::FramePacer;
use exa_game::window::{toggled_style, window_mode};

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
                    apply_launch_fullscreen,
                    toggle_windowed,
                    clear_keys_on_focus_loss,
                ),
            );
    }
}

/// The frame-rate cap the launcher asks for: positive is a fixed rate, a
/// negative value automatic, zero off (the default). The original's cap
/// only ever holds the loop back, so the default leaves the frame at the
/// display's or the GPU's pace.
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
        pacing.fps_cap = options.max_fps;
    }
}

/// Holds the frame to the configured rate, measuring from the previous
/// frame's start as the original does. With no cap, or while the display
/// rate is unknown, nothing happens.
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
    let refresh_hz = monitors
        .single()
        .ok()
        .and_then(|monitor| monitor.refresh_rate_millihertz)
        .map(|milli| (milli / 1000).max(1))
        .unwrap_or(0);
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

/// Clears the held keys and mouse buttons when the window loses focus, as
/// the original's focus message empties its 256-entry key table. Without it a
/// key held at the moment of the switch would stay down until pressed again.
fn clear_keys_on_focus_loss(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut keys: Option<ResMut<ButtonInput<KeyCode>>>,
    mut buttons: Option<ResMut<ButtonInput<MouseButton>>>,
    mut focused: Local<Option<bool>>,
) {
    let Ok(window) = windows.single() else { return };
    if *focused == Some(window.focused) {
        return;
    }
    let was = focused.replace(window.focused);
    if was == Some(true) && !window.focused {
        if let Some(keys) = keys.as_mut() {
            keys.clear();
        }
        if let Some(buttons) = buttons.as_mut() {
            buttons.clear();
        }
    }
}

/// Picks the exclusive display mode once the monitor is known: the requested
/// window size when the display lists it, otherwise the current mode. The
/// original reports an unsupported mode and keeps the current screen size.
fn apply_launch_fullscreen(
    options: Option<Res<Options>>,
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(options) = options else { return };
    if !options.fullscreen {
        *done = true;
        return;
    }
    let Ok(monitor) = monitors.single() else {
        return;
    };
    let (width, height) = options.size.unwrap_or((1600, 900));
    let requested = bevy::prelude::UVec2::new(width, height);
    let video_mode = monitor
        .video_modes
        .iter()
        .find(|mode| mode.physical_size == requested)
        .copied()
        .map(bevy::window::VideoModeSelection::Specific)
        .unwrap_or(bevy::window::VideoModeSelection::Current);
    if let Ok(mut window) = windows.single_mut() {
        window.mode = WindowMode::Fullscreen(bevy::window::MonitorSelection::Primary, video_mode);
    }
    *done = true;
}

/// Alt+Enter switches between a windowed frame and borderless full screen,
/// restoring the size and position the window had. The original intercepts
/// the system key, so releases do not repeat the toggle.
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

/// The window mode a launch full-screen flag asks for: the original changes
/// the display mode and creates an exclusive full-screen window. Winit maps
/// this to an exclusive mode where the platform has one and to borderless
/// full screen on Wayland.
pub fn launch_window_mode(fullscreen: bool) -> WindowMode {
    if fullscreen {
        window_mode(exa_game::window::WindowStyle::Exclusive)
    } else {
        WindowMode::Windowed
    }
}
