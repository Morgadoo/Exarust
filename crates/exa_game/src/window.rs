//! Window styles and display modes.
//!
//! Window styles and display modes.
//!
//! The original creates a windowed frame, a borderless popup or an exclusive
//! full-screen window whose display mode it changes first, with a 640 × 480
//! minimum and Alt+Enter toggling between windowed and borderless. Winit and
//! Bevy own the window; the rules that can be expressed through
//! [`bevy::window::Window`] are applied by `exarust::frame_loop`, and the
//! display-mode enumeration and fitting arithmetic are modelled here and
//! pinned by tests. Rules: `docs/PERFORMANCE.md`, `docs/UI.md` (Interface
//! scale).

use bevy::prelude::*;
use bevy::window::{MonitorSelection, VideoMode, VideoModeSelection, WindowMode};

/// The original's minimum window client size, in physical pixels.
pub const MIN_WINDOW_SIZE: UVec2 = UVec2::new(640, 480);

/// Modes narrower than this are not offered by the original's display-mode
/// list.
pub const MIN_DISPLAY_WIDTH: u32 = 640;

/// How a window is framed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowStyle {
    /// A resizable framed window.
    Windowed,
    /// A borderless window covering the display.
    Borderless,
    /// Exclusive full screen at a chosen display mode.
    Exclusive,
}

/// One display mode the original enumerates: a size and a refresh rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    /// Refresh rate in hertz; 1 means the driver did not report one.
    pub refresh_hz: u32,
}

/// The display modes, ordered by area as the original orders them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayModes {
    modes: Vec<DisplayMode>,
}

impl DisplayModes {
    /// Builds the list from a driver's enumeration: modes narrower than
    /// [`MIN_DISPLAY_WIDTH`] are dropped, a size is stored once with every
    /// refresh rate above 1 that the driver reports for it, and the list is
    /// sorted by pixel area (the original's ascending selection-sort order;
    /// equal areas keep their discovery order here).
    pub fn from_enumerated<'a>(modes: impl IntoIterator<Item = &'a VideoMode>) -> Self {
        let mut out: Vec<DisplayMode> = Vec::new();
        for mode in modes {
            let width = mode.physical_size.x;
            let height = mode.physical_size.y;
            if width < MIN_DISPLAY_WIDTH {
                continue;
            }
            let refresh_hz = mode.refresh_rate_millihertz / 1000;
            match out
                .iter_mut()
                .find(|m| m.width == width && m.height == height)
            {
                Some(existing) => {
                    if refresh_hz > 1 && !existing.has_refresh(refresh_hz) {
                        // The original records only one rate per size in its
                        // per-size list; the first is enough to pick a mode.
                        existing.refresh_hz = existing.refresh_hz.max(refresh_hz);
                    }
                }
                None => out.push(DisplayMode {
                    width,
                    height,
                    refresh_hz,
                }),
            }
        }
        out.sort_by_key(|m| m.width as u64 * m.height as u64);
        Self { modes: out }
    }

    /// The modes, widest last.
    pub fn modes(&self) -> &[DisplayMode] {
        &self.modes
    }

    /// The index of the mode matching the current screen size, if the driver
    /// lists it.
    pub fn index_of(&self, width: u32, height: u32) -> Option<usize> {
        self.modes
            .iter()
            .position(|m| m.width == width && m.height == height)
    }

    /// Whether a size is listed.
    pub fn contains(&self, width: u32, height: u32) -> bool {
        self.index_of(width, height).is_some()
    }
}

impl DisplayMode {
    fn has_refresh(&self, hz: u32) -> bool {
        self.refresh_hz == hz
    }
}

/// The swap's present mode for the vertical-sync setting and the adaptive
/// choice: off presents immediately, adaptive vertical sync lets a late
/// frame tear instead of waiting (`FifoRelaxed`), and plain vertical sync
/// waits for the display (`Fifo`). The original sets the swap interval to
/// 0, 1 or −1 respectively.
pub fn present_mode(vertical_sync: bool, adaptive: bool) -> bevy::window::PresentMode {
    use bevy::window::PresentMode;
    if !vertical_sync {
        PresentMode::AutoNoVsync
    } else if adaptive {
        // AutoVsync prefers FifoRelaxed, the swap-control-tear behaviour.
        PresentMode::AutoVsync
    } else {
        PresentMode::Fifo
    }
}

/// The Bevy window mode for a style. Exclusive full screen uses the display's
/// current mode (winit's video-mode selection); on Wayland, which has no
/// exclusive mode, Bevy falls back to a borderless window.
pub fn window_mode(style: WindowStyle) -> WindowMode {
    match style {
        WindowStyle::Windowed => WindowMode::Windowed,
        WindowStyle::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        WindowStyle::Exclusive => {
            WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
        }
    }
}

/// Where a window of `size` goes inside a work area, keeping it wholly
/// visible.
///
/// The original adds the frame's borders to the desired client size, drops a
/// size larger than the work area to the work area's, and centres a window
/// whose frame is smaller only in the borderless mode. This model takes the
/// resulting outer size and the work area: `center` is the borderless flag.
pub fn fit_window(
    desired: UVec2,
    work_origin: IVec2,
    work_size: UVec2,
    center: bool,
) -> (IVec2, UVec2) {
    let mut size = desired;
    let mut origin = work_origin;
    if size.x > work_size.x {
        size.x = work_size.x;
        origin.x = work_origin.x;
    } else if center {
        origin.x = work_origin.x + (work_size.x - size.x) as i32 / 2;
    }
    if size.y > work_size.y {
        size.y = work_size.y;
        origin.y = work_origin.y;
    } else if center {
        origin.y = work_origin.y + (work_size.y - size.y) as i32 / 2;
    }
    (origin, size)
}

/// The window a full-screen toggle returns to: Alt+Enter switches a windowed
/// frame to borderless full screen and back, restoring the saved client
/// size.
pub fn toggled_style(current: WindowMode) -> WindowStyle {
    match current {
        WindowMode::Windowed => WindowStyle::Borderless,
        _ => WindowStyle::Windowed,
    }
}

/// The window the game's display-mode setting asks for.
///
/// The original's "full screen" is exclusive; a mode the display does not
/// list makes it report the mode as unsupported and keep the current screen
/// size. The caller applies the result to a window's [`WindowMode`].
pub fn style_for_setting(fullscreen: bool, borderless: bool, exclusive: bool) -> WindowStyle {
    if !fullscreen {
        WindowStyle::Windowed
    } else if borderless || !exclusive {
        WindowStyle::Borderless
    } else {
        WindowStyle::Exclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video(width: u32, height: u32, hz: u32) -> VideoMode {
        VideoMode {
            physical_size: UVec2::new(width, height),
            bit_depth: 32,
            refresh_rate_millihertz: hz * 1000,
        }
    }

    #[test]
    fn display_modes_are_filtered_deduped_and_ordered_by_area() {
        let modes = [
            video(1920, 1080, 60),
            video(800, 600, 60),
            video(640, 480, 60),
            video(320, 200, 60),
            video(1920, 1080, 144),
            video(1280, 720, 60),
            video(1280, 720, 1),
        ];
        let list = DisplayModes::from_enumerated(&modes);
        assert_eq!(
            list.modes(),
            &[
                DisplayMode {
                    width: 640,
                    height: 480,
                    refresh_hz: 60
                },
                DisplayMode {
                    width: 800,
                    height: 600,
                    refresh_hz: 60
                },
                DisplayMode {
                    width: 1280,
                    height: 720,
                    refresh_hz: 60
                },
                DisplayMode {
                    width: 1920,
                    height: 1080,
                    refresh_hz: 144
                },
            ]
        );
        assert_eq!(list.index_of(1280, 720), Some(2));
        assert_eq!(list.index_of(1920, 1200), None);
        assert!(list.contains(640, 480));
        assert!(!list.contains(320, 200));
    }

    #[test]
    fn a_window_fits_the_work_area() {
        let origin = IVec2::new(10, 20);
        let work = UVec2::new(1920, 1040);
        // A smaller borderless window is centred.
        assert_eq!(
            fit_window(UVec2::new(1600, 900), origin, work, true),
            (IVec2::new(170, 90), UVec2::new(1600, 900))
        );
        // A framed window keeps its requested corner.
        assert_eq!(
            fit_window(UVec2::new(1600, 900), origin, work, false),
            (origin, UVec2::new(1600, 900))
        );
        // Too large: both axes clamp to the work area.
        assert_eq!(
            fit_window(UVec2::new(3000, 2000), origin, work, true),
            (origin, work)
        );
    }

    #[test]
    fn the_present_mode_follows_the_swap_interval() {
        use bevy::window::PresentMode;
        assert_eq!(present_mode(false, false), PresentMode::AutoNoVsync);
        assert_eq!(present_mode(true, false), PresentMode::Fifo);
        assert_eq!(present_mode(true, true), PresentMode::AutoVsync);
    }

    #[test]
    fn the_minimum_and_the_toggle_are_the_originals() {
        assert_eq!(MIN_WINDOW_SIZE, UVec2::new(640, 480));
        assert_eq!(toggled_style(WindowMode::Windowed), WindowStyle::Borderless);
        assert_eq!(
            toggled_style(WindowMode::BorderlessFullscreen(MonitorSelection::Primary)),
            WindowStyle::Windowed
        );
        assert_eq!(
            window_mode(WindowStyle::Exclusive),
            WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
        );
        assert_eq!(style_for_setting(false, false, true), WindowStyle::Windowed);
        assert_eq!(style_for_setting(true, false, true), WindowStyle::Exclusive);
        assert_eq!(style_for_setting(true, true, true), WindowStyle::Borderless);
        assert_eq!(
            style_for_setting(true, false, false),
            WindowStyle::Borderless
        );
    }
}
