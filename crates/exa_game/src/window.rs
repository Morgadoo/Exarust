//! Window styles and display modes.
//!
//! The original creates a framed window, or for full screen a popup that
//! covers the desktop, changing the display mode first only when its
//! `ScreenWidth`/`ScreenHeight` settings ask for a size other than the
//! desktop's. Its window has a 640 × 480 minimum track size, and Alt+Enter
//! toggles between the framed window and full screen. Winit and Bevy own the
//! window; the rules that can be expressed through [`bevy::window::Window`]
//! are applied by `hw_config` (the settings' full screen and vertical sync)
//! and `exarust::frame_loop` (pacing, Alt+Enter), and the display-mode list
//! and window fitting are modelled here and pinned by tests. Rules:
//! `docs/PERFORMANCE.md` (Frame pacing), `docs/UI.md` (Settings options).

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, VideoMode, VideoModeSelection, WindowMode};

/// The original's minimum track size. Win32 measures it on the outer
/// window, borders and caption included; winit's resize constraints apply
/// to the inner size, so Exarust's window is that much larger at the limit.
pub const MIN_WINDOW_SIZE: UVec2 = UVec2::new(640, 480);

/// Modes narrower than this are not offered by the original's display-mode
/// list.
pub const MIN_DISPLAY_WIDTH: u32 = 640;

/// Rates below this are reported as unknown (0) by the original's refresh
/// rate query.
pub const MIN_KNOWN_REFRESH_HZ: u32 = 30;

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

/// One size the original's display-mode list holds, with every refresh
/// rate the driver reported for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    /// The rates in discovery order: the first as reported (even 0 or 1),
    /// later ones only when above 1 and not already listed.
    pub refresh_rates: Vec<u32>,
}

/// The display modes, ordered by area as the original orders them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayModes {
    modes: Vec<DisplayMode>,
}

impl DisplayModes {
    /// Builds the list from a driver's enumeration, as the original does:
    /// modes narrower than [`MIN_DISPLAY_WIDTH`] are skipped, a new size is
    /// stored with its first rate and a known size gains each further rate
    /// above 1 it does not list yet. The list is then ordered by pixel area
    /// with the original's exchange sort: each slot in turn swaps with every
    /// later entry of smaller area, so equal areas need not keep their
    /// discovery order.
    pub fn from_enumerated<'a>(modes: impl IntoIterator<Item = &'a VideoMode>) -> Self {
        let mut out: Vec<DisplayMode> = Vec::new();
        for mode in modes {
            let width = mode.physical_size.x;
            let height = mode.physical_size.y;
            if width < MIN_DISPLAY_WIDTH {
                continue;
            }
            // Win32 reports whole hertz; winit's millihertz are truncated.
            let refresh_hz = mode.refresh_rate_millihertz / 1000;
            match out
                .iter_mut()
                .find(|m| m.width == width && m.height == height)
            {
                Some(existing) => {
                    if refresh_hz > 1 && !existing.refresh_rates.contains(&refresh_hz) {
                        existing.refresh_rates.push(refresh_hz);
                    }
                }
                None => out.push(DisplayMode {
                    width,
                    height,
                    refresh_rates: vec![refresh_hz],
                }),
            }
        }
        let area = |m: &DisplayMode| i64::from(m.width) * i64::from(m.height);
        for i in 0..out.len() {
            for j in i + 1..out.len() {
                if area(&out[j]) < area(&out[i]) {
                    out.swap(i, j);
                }
            }
        }
        Self { modes: out }
    }

    /// The modes, largest last.
    pub fn modes(&self) -> &[DisplayMode] {
        &self.modes
    }

    /// The index of the first mode of this size, if the driver lists it.
    pub fn index_of(&self, width: u32, height: u32) -> Option<usize> {
        self.modes
            .iter()
            .position(|m| m.width == width && m.height == height)
    }

    /// The desktop mode's index the original keeps: the first entry of the
    /// desktop's size, else 0.
    pub fn current_index(&self, desktop: UVec2) -> usize {
        self.index_of(desktop.x, desktop.y).unwrap_or(0)
    }

    /// Whether a size is listed.
    pub fn contains(&self, width: u32, height: u32) -> bool {
        self.index_of(width, height).is_some()
    }
}

/// The display's refresh rate as the original's pacing sees it: whole hertz,
/// and 0 (unknown) below [`MIN_KNOWN_REFRESH_HZ`] or when none is reported.
pub fn display_refresh_hz(refresh_rate_millihertz: Option<u32>) -> u32 {
    match refresh_rate_millihertz.map(|milli| milli / 1000) {
        Some(hz) if hz >= MIN_KNOWN_REFRESH_HZ => hz,
        _ => 0,
    }
}

/// The swap's present mode for the vertical-sync setting: off presents
/// immediately (swap interval 0), on waits for the display (`Fifo`, swap
/// interval 1). The original's setter would pick the adaptive interval −1
/// (`FifoRelaxed`, which `AutoVsync` prefers) when its adaptive switch is
/// set, but nothing in the shipped executable sets that switch, so play
/// always uses interval 1; `adaptive` is kept for that branch.
pub fn present_mode(vertical_sync: bool, adaptive: bool) -> PresentMode {
    if !vertical_sync {
        PresentMode::AutoNoVsync
    } else if adaptive {
        PresentMode::AutoVsync
    } else {
        PresentMode::Fifo
    }
}

/// The Bevy window mode for a style on the primary display. Exclusive full
/// screen keeps the display's current mode here; [`fullscreen_mode`] picks a
/// specific one.
pub fn window_mode(style: WindowStyle) -> WindowMode {
    match style {
        WindowStyle::Windowed => WindowMode::Windowed,
        WindowStyle::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        WindowStyle::Exclusive => {
            WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
        }
    }
}

/// The full-screen window the original creates for its `ScreenWidth` and
/// `ScreenHeight` settings (0 when unset).
///
/// When either is set and differs from the desktop's size, the original
/// changes the display mode to that size and covers it; a mode the display
/// refuses is reported ("Display mode not supported.") and the desktop's
/// size is kept. Otherwise the window simply covers the desktop. Exarust
/// maps the first case to exclusive full screen at the listed video mode
/// (`modes`, the primary monitor's) and every other case to a borderless
/// window. `desktop` is `None` when the primary monitor is unknown (Wayland
/// has no primary monitor), which gives the borderless window.
pub fn fullscreen_mode(
    screen_size: (i32, i32),
    desktop: Option<UVec2>,
    modes: &[VideoMode],
) -> WindowMode {
    let borderless = WindowMode::BorderlessFullscreen(MonitorSelection::Primary);
    let Some(desktop) = desktop else {
        return borderless;
    };
    let (width, height) = screen_size;
    let differs = (width != 0 && width as i64 != i64::from(desktop.x))
        || (height != 0 && height as i64 != i64::from(desktop.y));
    if !differs || width <= 0 || height <= 0 {
        return borderless;
    }
    let size = UVec2::new(width as u32, height as u32);
    modes
        .iter()
        .find(|mode| mode.physical_size == size)
        .map_or(borderless, |mode| {
            WindowMode::Fullscreen(
                MonitorSelection::Primary,
                VideoModeSelection::Specific(*mode),
            )
        })
}

/// Where a framed window goes inside a work area.
///
/// `outer` is the window's size with its borders and caption (the original
/// adds them from the system metrics before calling this). An axis larger
/// than the work area takes the work area's size and its left or top edge;
/// otherwise, when `center` is set, the window is placed half the spare
/// space in, measured from 0 rather than from the work area's edge (the
/// original adds no origin); otherwise `position` is kept. The original
/// centres only a window that has never been framed before, at its default
/// size; a restored window keeps its saved position.
pub fn fit_window(
    position: IVec2,
    outer: UVec2,
    work_origin: IVec2,
    work_size: UVec2,
    center: bool,
) -> (IVec2, UVec2) {
    let mut size = outer;
    let mut origin = position;
    if i64::from(work_size.x) < i64::from(size.x) {
        size.x = work_size.x;
        origin.x = work_origin.x;
    } else if center {
        origin.x = ((work_size.x - size.x) / 2) as i32;
    }
    if i64::from(work_size.y) < i64::from(size.y) {
        size.y = work_size.y;
        origin.y = work_origin.y;
    } else if center {
        origin.y = ((work_size.y - size.y) / 2) as i32;
    }
    (origin, size)
}

/// The window an Alt+Enter toggle switches to: a framed window goes full
/// screen and anything else back to the framed window, which gets its saved
/// size and position. (The original's full screen is exclusive only when
/// its settings changed the display mode; the toggle keeps that choice.)
pub fn toggled_style(current: WindowMode) -> WindowStyle {
    match current {
        WindowMode::Windowed => WindowStyle::Borderless,
        _ => WindowStyle::Windowed,
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

    fn mode(width: u32, height: u32, rates: &[u32]) -> DisplayMode {
        DisplayMode {
            width,
            height,
            refresh_rates: rates.to_vec(),
        }
    }

    #[test]
    fn display_modes_are_filtered_merged_and_ordered_by_area() {
        let modes = [
            video(1920, 1080, 60),
            video(800, 600, 60),
            video(640, 480, 60),
            video(320, 200, 60),
            video(1920, 1080, 144),
            video(1280, 720, 1),
            video(1280, 720, 60),
            video(1920, 1080, 60),
            video(1280, 720, 1),
        ];
        let list = DisplayModes::from_enumerated(&modes);
        assert_eq!(
            list.modes(),
            &[
                mode(640, 480, &[60]),
                mode(800, 600, &[60]),
                // The first rate is kept even when it is 1; a later 1 is not.
                mode(1280, 720, &[1, 60]),
                mode(1920, 1080, &[60, 144]),
            ]
        );
        assert_eq!(list.index_of(1280, 720), Some(2));
        assert_eq!(list.index_of(1920, 1200), None);
        assert_eq!(list.current_index(UVec2::new(1920, 1080)), 3);
        assert_eq!(list.current_index(UVec2::new(2560, 1440)), 0);
        assert!(list.contains(640, 480));
        assert!(!list.contains(320, 200));
    }

    #[test]
    fn equal_areas_follow_the_exchange_sort() {
        // Slot 0 takes the smaller 800 x 600 from the end, which pushes
        // 1200 x 800 back behind 960 x 1000 of the same area.
        let modes = [
            video(1200, 800, 60),
            video(960, 1000, 60),
            video(800, 600, 60),
        ];
        let list = DisplayModes::from_enumerated(&modes);
        let sizes: Vec<(u32, u32)> = list.modes().iter().map(|m| (m.width, m.height)).collect();
        assert_eq!(sizes, [(800, 600), (960, 1000), (1200, 800)]);
    }

    #[test]
    fn the_refresh_rate_is_unknown_below_30_hz() {
        assert_eq!(display_refresh_hz(Some(143_998)), 143);
        assert_eq!(display_refresh_hz(Some(59_940)), 59);
        assert_eq!(display_refresh_hz(Some(30_000)), 30);
        assert_eq!(display_refresh_hz(Some(29_999)), 0);
        assert_eq!(display_refresh_hz(None), 0);
    }

    #[test]
    fn a_window_fits_the_work_area() {
        let work_origin = IVec2::new(10, 20);
        let work = UVec2::new(1920, 1040);
        // A first framed window is centred from 0, not from the work area's
        // corner: (1920 - 1600) / 2 = 160, (1040 - 900) / 2 = 70.
        assert_eq!(
            fit_window(IVec2::ZERO, UVec2::new(1600, 900), work_origin, work, true),
            (IVec2::new(160, 70), UVec2::new(1600, 900))
        );
        // A restored window keeps its position.
        assert_eq!(
            fit_window(
                IVec2::new(300, 200),
                UVec2::new(1600, 900),
                work_origin,
                work,
                false
            ),
            (IVec2::new(300, 200), UVec2::new(1600, 900))
        );
        // Too large: each axis takes the work area and its edge.
        assert_eq!(
            fit_window(
                IVec2::new(5, 5),
                UVec2::new(3000, 900),
                work_origin,
                work,
                true
            ),
            (IVec2::new(10, 70), UVec2::new(1920, 900))
        );
    }

    #[test]
    fn the_present_mode_follows_the_swap_interval() {
        assert_eq!(present_mode(false, false), PresentMode::AutoNoVsync);
        assert_eq!(present_mode(true, false), PresentMode::Fifo);
        assert_eq!(present_mode(true, true), PresentMode::AutoVsync);
    }

    #[test]
    fn full_screen_changes_the_mode_only_for_another_size() {
        let desktop = Some(UVec2::new(2560, 1440));
        let modes = [video(2560, 1440, 144), video(1920, 1080, 60)];
        let borderless = WindowMode::BorderlessFullscreen(MonitorSelection::Primary);
        // Unset, or the desktop's size: a window over the desktop.
        assert_eq!(fullscreen_mode((0, 0), desktop, &modes), borderless);
        assert_eq!(fullscreen_mode((2560, 1440), desktop, &modes), borderless);
        // Another listed size: that display mode.
        assert_eq!(
            fullscreen_mode((1920, 1080), desktop, &modes),
            WindowMode::Fullscreen(
                MonitorSelection::Primary,
                VideoModeSelection::Specific(modes[1])
            )
        );
        // Unlisted, half set, or no primary monitor: the desktop's size.
        assert_eq!(fullscreen_mode((1600, 900), desktop, &modes), borderless);
        assert_eq!(fullscreen_mode((1920, 0), desktop, &modes), borderless);
        assert_eq!(fullscreen_mode((1920, 1080), None, &modes), borderless);
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
    }
}
