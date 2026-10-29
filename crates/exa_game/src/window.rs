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

/// Whether Exarust asks [`present_mode`] for the adaptive interval with
/// vertical sync on. The original uses swap interval 1 (plain `Fifo`), but
/// Exarust's frames still run slower than the display, and plain `Fifo`
/// holds every late frame to the next blank: in `play exanima01` it cost
/// about 4 ms a frame (36.7 against 33.0 ms over three interleaved runs
/// each). `AutoVsync` waits for the display when a frame is on time and
/// presents a late one at once (`FifoRelaxed` where the driver has it).
pub const ADAPTIVE_VSYNC: bool = true;

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

/// The title and text the original reports (level 4, the on-screen
/// overlay and the log file) when the display refuses the full-screen
/// window's mode; the desktop's size is kept.
pub const DISPLAY_MODE_REFUSED: (&str, &str) = ("Error", "Display mode not supported.");

/// Whether [`fullscreen_mode`] falls back to the desktop's size because the
/// display does not list the settings' size: the case the original reports
/// with [`DISPLAY_MODE_REFUSED`]. Exarust takes a mode the display does not
/// list as refused, where the original asks the driver.
pub fn display_mode_refused(
    screen_size: (i32, i32),
    desktop: Option<UVec2>,
    modes: &[VideoMode],
) -> bool {
    let Some(desktop) = desktop else {
        return false;
    };
    let (width, height) = screen_size;
    let differs = (width != 0 && width as i64 != i64::from(desktop.x))
        || (height != 0 && height as i64 != i64::from(desktop.y));
    if !differs {
        return false;
    }
    !matches!(
        fullscreen_mode(screen_size, Some(desktop), modes),
        WindowMode::Fullscreen(..)
    )
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

/// Whether a full-screen window minimises when another application takes
/// the activation: the original's switch for it is on in the shipped
/// executable.
pub const MINIMIZE_FULL_SCREEN_ON_DEACTIVATE: bool = true;

/// Whether the window minimises on an activation change, as the original's
/// window procedure decides: only with the switch on, only while full
/// screen (borderless or exclusive) and only when deactivated.
pub fn minimizes_on_deactivate(switch: bool, full_screen: bool, activated: bool) -> bool {
    switch && full_screen && !activated
}

/// How many pointer moves a warp stays in force for.
pub const WARP_HOLD_MOVES: i32 = 4;
/// A move that lands closer than this to the warp's target on both axes
/// (in pixels) ends the hold.
pub const WARP_SETTLE_PX: i64 = 41;

/// The original's warp of the pointer while the game moves it itself: for
/// the next few pointer moves the position read back is the warp's target,
/// and a move that lands away from it puts the pointer back there, until a
/// move lands near the target or the moves run out. This covers the
/// moves the platform may still deliver from before the warp.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WarpHold {
    /// Moves left; the hold is off at zero or below.
    pub remaining: i32,
    /// The warp's target in client pixels.
    pub target: IVec2,
}

impl WarpHold {
    /// Warps to `to` (client pixels). A target inside the client area
    /// (`0 ≤ x < width`, `0 ≤ y < height`) starts a hold of
    /// [`WARP_HOLD_MOVES`]; one outside only moves the pointer. Either way
    /// the pointer is moved to `to` (the original sets it six times over).
    pub fn warp(&mut self, to: IVec2, client: IVec2) -> IVec2 {
        if to.x >= 0 && to.x < client.x && to.y >= 0 && to.y < client.y {
            self.target = to;
            self.remaining = WARP_HOLD_MOVES;
        }
        to
    }

    /// The pointer position the game reads: the target while the hold is
    /// on, else the platform's.
    pub fn read(&self, platform: IVec2) -> IVec2 {
        if self.remaining >= 1 {
            self.target
        } else {
            platform
        }
    }

    /// A pointer move reported at `at`: with the hold on, a move closer
    /// than [`WARP_SETTLE_PX`] to the target on both axes ends it;
    /// otherwise the pointer is put back on the target (returned) and one
    /// move is used up.
    pub fn moved(&mut self, at: IVec2) -> Option<IVec2> {
        if self.remaining < 1 {
            return None;
        }
        let dx = (i64::from(self.target.x) - i64::from(at.x)).abs();
        let dy = (i64::from(self.target.y) - i64::from(at.y)).abs();
        if dx < WARP_SETTLE_PX && dy < WARP_SETTLE_PX {
            self.remaining = 0;
            return None;
        }
        self.remaining -= 1;
        Some(self.target)
    }
}

/// An unbounded side of a pointer clip rectangle.
pub const CLIP_UNBOUNDED: i32 = -0x7fff_ffff;

/// The original's pointer confinement for GUI drags, with its endless
/// wrap: while a clip is set the pointer is held inside it, and a pointer
/// that reaches the screen's edge is moved to the opposite side while a
/// virtual offset of whole screen sizes keeps the position the game sees
/// growing. Screen coordinates throughout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerClip {
    /// The screen's size.
    pub screen: IVec2,
    /// The clip rectangle in screen pixels (left, top, right, bottom).
    pub clip: [i32; 4],
    /// Whether a clip is set.
    pub active: bool,
    /// The virtual offset: whole screen widths and heights wrapped so far.
    pub offset: IVec2,
}

impl PointerClip {
    /// Sets the clip from a client rectangle (left, top, right, bottom)
    /// placed at `client_origin` on the screen, or clears it. An unbounded
    /// side ([`CLIP_UNBOUNDED`]) becomes one pixel inside the screen's edge
    /// (1, 1, width − 1, height − 1). Either way the virtual offset starts
    /// again from zero. Returns the rectangle the platform confines the
    /// pointer to (`None`: free); a cleared clip leaves the full-screen
    /// focus confinement to the caller, as the original re-applies it.
    pub fn set(&mut self, rect: Option<[i32; 4]>, client_origin: IVec2) -> Option<[i32; 4]> {
        // The original resets the offset from a pair that nothing writes.
        self.offset = IVec2::ZERO;
        let Some(rect) = rect else {
            self.active = false;
            return None;
        };
        let mut clip = [
            rect[0].wrapping_add(client_origin.x),
            rect[1].wrapping_add(client_origin.y),
            rect[2].wrapping_add(client_origin.x),
            rect[3].wrapping_add(client_origin.y),
        ];
        if rect[0] == CLIP_UNBOUNDED {
            clip[0] = 1;
        }
        if rect[2] == CLIP_UNBOUNDED {
            clip[2] = self.screen.x - 1;
        }
        if rect[1] == CLIP_UNBOUNDED {
            clip[1] = 1;
        }
        if rect[3] == CLIP_UNBOUNDED {
            clip[3] = self.screen.y - 1;
        }
        self.clip = clip;
        self.active = true;
        self.confinement()
    }

    /// The rectangle the pointer is confined to: the screen moved by the
    /// virtual offset, clipped to the clip rectangle by the engine's
    /// rectangle clip, moved back. `None` when they are apart or the
    /// overlap has no area (the pointer is then free).
    pub fn confinement(&self) -> Option<[i32; 4]> {
        use exa_render::view::Rect;
        let mut screen = Rect::from_edges(
            self.offset.x,
            self.offset.y,
            self.offset.x + self.screen.x,
            self.offset.y + self.screen.y,
        );
        let clip = Rect::from_edges(self.clip[0], self.clip[1], self.clip[2], self.clip[3]);
        if !screen.clip_to(clip) {
            return None;
        }
        let edges = [
            screen.x - self.offset.x,
            screen.y - self.offset.y,
            screen.right() - self.offset.x,
            screen.bottom() - self.offset.y,
        ];
        let area = i64::from(edges[2] - edges[0]) * i64::from(edges[3] - edges[1]);
        (area >= 1).then_some(edges)
    }

    /// The pointer at screen point `at` while the clip is set: a pointer on
    /// the left edge goes to two pixels short of the right one and the
    /// offset loses a screen width, one on the right edge goes to x = 1 and
    /// the offset gains a width, and likewise for the top and bottom (one
    /// edge a call, horizontal first). Returns the position the game sees
    /// (the point plus the offset) and, after a wrap, where the platform
    /// pointer is moved to.
    pub fn wrap(&mut self, at: IVec2) -> (IVec2, Option<IVec2>) {
        let mut point = at;
        let wrapped = if point.x == 0 {
            self.offset.x -= self.screen.x;
            point.x = self.screen.x - 2;
            true
        } else if i64::from(point.x) == i64::from(self.screen.x) - 1 {
            self.offset.x += self.screen.x;
            point.x = 1;
            true
        } else if point.y == 0 {
            self.offset.y -= self.screen.y;
            point.y = self.screen.y - 2;
            true
        } else if i64::from(point.y) == i64::from(self.screen.y) - 1 {
            self.offset.y += self.screen.y;
            point.y = 1;
            true
        } else {
            false
        };
        (point + self.offset, wrapped.then_some(point))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_the_display_does_not_list_is_reported() {
        let desktop = Some(UVec2::new(1920, 1080));
        let modes = [video(1280, 720, 60), video(1920, 1080, 60)];
        assert!(!display_mode_refused((0, 0), desktop, &modes));
        assert!(!display_mode_refused((1920, 1080), desktop, &modes));
        assert!(!display_mode_refused((1280, 720), desktop, &modes));
        assert!(display_mode_refused((1024, 768), desktop, &modes));
        assert!(display_mode_refused((-5, 768), desktop, &modes));
        assert!(!display_mode_refused((1024, 768), None, &modes));
        assert_eq!(DISPLAY_MODE_REFUSED.1, "Display mode not supported.");
    }

    #[test]
    fn a_full_screen_window_minimises_when_deactivated() {
        assert!(minimizes_on_deactivate(true, true, false));
        assert!(!minimizes_on_deactivate(true, true, true));
        assert!(!minimizes_on_deactivate(true, false, false));
        assert!(!minimizes_on_deactivate(false, true, false));
        const { assert!(MINIMIZE_FULL_SCREEN_ON_DEACTIVATE) };
    }

    #[test]
    fn a_warp_holds_the_pointer_until_a_move_lands_near_it() {
        let mut hold = WarpHold::default();
        let client = IVec2::new(800, 600);
        // Outside the client area: moved, no hold.
        assert_eq!(hold.warp(IVec2::new(800, 10), client), IVec2::new(800, 10));
        assert_eq!(hold.remaining, 0);
        assert_eq!(hold.read(IVec2::new(5, 5)), IVec2::new(5, 5));
        hold.warp(IVec2::new(400, 300), client);
        assert_eq!(hold.remaining, 4);
        assert_eq!(hold.read(IVec2::new(5, 5)), IVec2::new(400, 300));
        // 41 px away on one axis: put back, one move used.
        assert_eq!(hold.moved(IVec2::new(441, 300)), Some(IVec2::new(400, 300)));
        assert_eq!(hold.remaining, 3);
        // 40 px on both axes: settled.
        assert_eq!(hold.moved(IVec2::new(360, 340)), None);
        assert_eq!(hold.remaining, 0);
        assert_eq!(hold.moved(IVec2::new(0, 0)), None);
        // Four stale moves use the hold up.
        hold.warp(IVec2::new(0, 0), client);
        for left in (0..4).rev() {
            assert_eq!(hold.moved(IVec2::new(100, 100)), Some(IVec2::ZERO));
            assert_eq!(hold.remaining, left);
        }
        assert_eq!(hold.read(IVec2::new(100, 100)), IVec2::new(100, 100));
    }

    #[test]
    fn a_pointer_clip_confines_and_wraps_at_the_screen_edges() {
        let mut clip = PointerClip {
            screen: IVec2::new(1920, 1080),
            ..Default::default()
        };
        // A client rectangle at (100, 50) on the screen.
        assert_eq!(
            clip.set(Some([10, 20, 210, 120]), IVec2::new(100, 50)),
            Some([110, 70, 310, 170])
        );
        // Every side unbounded: one pixel in from the screen's edges.
        let u = CLIP_UNBOUNDED;
        assert_eq!(
            clip.set(Some([u, u, u, u]), IVec2::new(100, 50)),
            Some([1, 1, 1919, 1079])
        );
        // Off the left edge: two short of the right, a screen width lost.
        assert_eq!(
            clip.wrap(IVec2::new(0, 500)),
            (IVec2::new(1918 - 1920, 500), Some(IVec2::new(1918, 500)))
        );
        assert_eq!(clip.offset, IVec2::new(-1920, 0));
        // The confinement follows the offset: apart from the clip now.
        assert_eq!(clip.confinement(), None);
        // The right edge wins over the top edge in the same call.
        let (seen, moved) = clip.wrap(IVec2::new(1919, 0));
        assert_eq!(moved, Some(IVec2::new(1, 0)));
        assert_eq!(seen, IVec2::new(1, 0));
        assert_eq!(clip.offset, IVec2::ZERO);
        assert_eq!(clip.wrap(IVec2::new(5, 1079)).1, Some(IVec2::new(5, 1)));
        assert_eq!(clip.offset, IVec2::new(0, 1080));
        assert_eq!(clip.wrap(IVec2::new(5, 6)), (IVec2::new(5, 1086), None));
        // Clearing frees the pointer and restarts the offset.
        assert_eq!(clip.set(None, IVec2::ZERO), None);
        assert!(!clip.active);
        assert_eq!(clip.offset, IVec2::ZERO);
        // A zero-area overlap leaves the pointer free.
        assert_eq!(clip.set(Some([0, 0, 0, 100]), IVec2::ZERO), None);
    }

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
