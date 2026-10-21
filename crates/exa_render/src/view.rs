//! The 3D view's projection, viewport and super-sampling rules.
//!
//! The original builds its own perspective frustum from the vertical field of
//! view, draws a frame in up to eight jittered sub-views into a shared
//! off-screen buffer, resolves them to the window and post-processes at
//! window resolution. It also keeps a seven-deep viewport and scissor stack
//! for nested targets. Exarust renders the scene with Bevy cameras and
//! resolves an ordered 2 × 2 grid; the constants and the arithmetic that the
//! original uses are kept here, modelled and pinned by tests, and the
//! differences are recorded in `docs/RENDERING.md`.
//!
//! Rules: `docs/RENDERING.md` (Supersampling, Post-processing).

use bevy::prelude::{IVec2, UVec2};

/// The original's super-sampling levels, by internal level. The settings
/// slider's 4× choice is internal level 3.
pub const MAX_SUPERSAMPLE_LEVEL: u8 = 4;
/// Sub-views drawn at each level (from the level's first jitter entry).
const SUBVIEW_COUNTS: [u8; 5] = [1, 2, 3, 4, 8];
/// The first jitter entry each level uses in [`JITTERS`].
const JITTER_OFFSETS: [usize; 5] = [0, 1, 3, 6, 10];
/// The sharpen strength's base at the 4× setting, before the level term.
pub const SHARPEN_BASE: f32 = 1.5;

/// Sub-pixel jitter of every sub-view, in pixels: x and y offsets from the
/// window pixel's centre. The table is the original's, in level order.
pub const JITTERS: [(f32, f32); 18] = [
    (0.0, 0.0),
    (0.246_49, 0.249_999),
    (-0.246_49, -0.249_999),
    (0.117_148, -0.117_57),
    (0.256_263, 0.368_119),
    (-0.373_411, -0.250_55),
    (-0.292_626, -0.149_945),
    (0.296_924, 0.149_994),
    (0.203_849, -0.353_78),
    (-0.208_147, 0.353_73),
    (0.164_216, -0.054_399),
    (-0.183_79, 0.082_102),
    (0.102_254, 0.299_133),
    (-0.079_263, -0.317_383),
    (0.286_438, -0.393_495),
    (-0.334_818, 0.435_331),
    (0.459_462, 0.141_54),
    (-0.414_498, -0.192_829),
];

/// One super-sampling level: how many jittered sub-views it draws, where
/// its jitters start, the render scale it draws them at and the sharpen
/// strength it gives the final pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupersampleLevel(pub u8);

impl Default for SupersampleLevel {
    /// The original's 4× setting.
    fn default() -> Self {
        Self(3)
    }
}

impl SupersampleLevel {
    /// The level clamped to the ones the original has.
    pub fn clamped(self) -> Self {
        Self(self.0.min(MAX_SUPERSAMPLE_LEVEL))
    }

    /// How many sub-views the level draws.
    pub fn subviews(self) -> usize {
        SUBVIEW_COUNTS[self.clamped().0 as usize] as usize
    }

    /// The level's sub-pixel jitters.
    pub fn jitters(self) -> &'static [(f32, f32)] {
        let level = self.clamped().0 as usize;
        let first = JITTER_OFFSETS[level];
        &JITTERS[first..first + SUBVIEW_COUNTS[level] as usize]
    }

    /// The share of the window each sub-view covers on each axis: 1, 3/4,
    /// 2/3, 3/5 or 1/2, truncated to whole pixels as the original does.
    pub fn render_scale(self) -> f32 {
        match self.clamped().0 {
            0 => 1.0,
            1 => 0.75,
            2 => 2.0 / 3.0,
            3 => 0.6,
            _ => 0.5,
        }
    }

    /// The sharpen strength the final pass gets: zero at level 0, then
    /// 1.5 × (0.075·level + 0.4); 0.9375 at the 4× setting.
    pub fn sharpen(self) -> f32 {
        let level = self.clamped().0;
        if level == 0 {
            0.0
        } else {
            SHARPEN_BASE * (level as f32 * 0.075 + 0.4)
        }
    }

    /// The size a view of `window` pixels draws its sub-views at, on each
    /// axis (`floor(size × scale)`, at least one pixel).
    pub fn render_size(self, window: UVec2) -> UVec2 {
        let scale = self.render_scale();
        UVec2::new(
            ((window.x as f32 * scale) as u32).max(1),
            ((window.y as f32 * scale) as u32).max(1),
        )
    }

    /// The shared buffer's size: twice each sub-view's size, the quadrants
    /// the sub-views are drawn into.
    pub fn buffer_size(self, window: UVec2) -> UVec2 {
        (self.render_size(window) * 2).max(UVec2::ONE)
    }
}

/// The perspective frustum's half extents at the near plane for a vertical
/// field of view in degrees and a viewport aspect ratio.
///
/// The original computes the half height in double precision from the
/// vertical field of view and the near distance (`near · tan(fov/2)`) and
/// the half width as that times the width/height ratio.
pub fn frustum_half_extents(fov_y_degrees: f32, aspect: f32, near: f32) -> (f32, f32) {
    let half = fov_y_degrees.to_radians() as f64 * 0.5;
    let height = near as f64 * half.tan();
    ((height * aspect as f64) as f32, height as f32)
}

/// An integer rectangle in window space, left/top inclusive and right/bottom
/// exclusive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// The empty rectangle.
    pub const EMPTY: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    /// A rectangle from its four edges.
    pub const fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }

    /// The right edge.
    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    /// The bottom edge.
    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    /// Whether the rectangle has no area.
    pub fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    /// Clips this rectangle to `other` in place, the original's rectangle
    /// clip: a rectangle touching the clip region by an edge only has no
    /// area and becomes [`Rect::EMPTY`]. Returns whether any area is left.
    ///
    /// The edges are compared exactly (`other.left < self.left`,
    /// `other.right < self.right`, and so on), so the empty result is stored
    /// as four zeroes.
    pub fn clip_to(&mut self, other: Rect) -> bool {
        if other.x < self.x {
            if other.right() < self.x {
                *self = Self::EMPTY;
                return false;
            }
            if other.right() < self.right() {
                self.width = other.right() - self.x;
            }
        } else {
            if self.right() <= other.x {
                *self = Self::EMPTY;
                return false;
            }
            self.x = other.x;
            if other.right() < self.right() {
                self.width = other.right() - self.x;
            }
        }
        if other.y < self.y {
            if other.bottom() < self.y {
                *self = Self::EMPTY;
                return false;
            }
            if other.bottom() < self.bottom() {
                self.height = other.bottom() - self.y;
            }
        } else {
            if self.bottom() <= other.y {
                *self = Self::EMPTY;
                return false;
            }
            self.y = other.y;
            if other.bottom() < self.bottom() {
                self.height = other.bottom() - self.y;
            }
        }
        true
    }
}

/// The deepest viewport and scissor stack the original keeps.
pub const VIEWPORT_DEPTH: usize = 7;

/// The original's framebuffer viewport stack.
///
/// A target starts with the whole target as its viewport. A pushed viewport
/// is placed at an offset from the current viewport's top-left corner and
/// clipped by the current scissor region (the whole target at the root). The
/// stack is seven deep; a push past that, or a pop from the root, changes
/// nothing and returns false. Exarust's cameras carry a single viewport;
/// nested viewports are a Bevy UI/clip concern.
#[derive(Clone, Debug)]
pub struct ViewportStack {
    target: Rect,
    rects: Vec<Rect>,
    regions: Vec<Rect>,
}

impl ViewportStack {
    /// A stack whose root viewport and region are the whole `target`.
    pub fn new(target: UVec2) -> Self {
        let rect = Rect {
            x: 0,
            y: 0,
            width: target.x as i32,
            height: target.y as i32,
        };
        Self {
            target: rect,
            rects: vec![rect],
            regions: vec![rect],
        }
    }

    /// The target rectangle.
    pub fn target(&self) -> Rect {
        self.target
    }

    /// The viewport in force.
    pub fn viewport(&self) -> Rect {
        *self.rects.last().unwrap_or(&self.target)
    }

    /// The scissor region in force.
    pub fn region(&self) -> Rect {
        *self.regions.last().unwrap_or(&self.target)
    }

    /// How many viewports are pushed.
    pub fn depth(&self) -> usize {
        self.rects.len()
    }

    /// Pushes a viewport `width` × `height` at (`x`, `y`) from the current
    /// viewport's corner, clipped by the current region. Returns false when
    /// the stack is full or the clipped viewport is empty.
    pub fn push_viewport(&mut self, x: i32, y: i32, width: i32, height: i32) -> bool {
        if self.rects.len() >= VIEWPORT_DEPTH {
            return false;
        }
        let current = self.viewport();
        let mut rect = Rect {
            x: current.x + x,
            y: current.y + y,
            width,
            height,
        };
        if !rect.clip_to(self.region()) || rect.is_empty() {
            return false;
        }
        self.rects.push(rect);
        true
    }

    /// Pops the current viewport. Returns false at the root.
    pub fn pop_viewport(&mut self) -> bool {
        if self.rects.len() <= 1 {
            return false;
        }
        self.rects.pop();
        true
    }

    /// Pushes a scissor region at (`x`, `y`) from the current viewport's
    /// corner, intersected with the current region. Returns false when the
    /// stack is full or the intersection is empty.
    pub fn push_region(&mut self, x: i32, y: i32, width: i32, height: i32) -> bool {
        if self.regions.len() >= VIEWPORT_DEPTH {
            return false;
        }
        let current = self.viewport();
        let mut rect = Rect {
            x: current.x + x,
            y: current.y + y,
            width,
            height,
        };
        if !rect.clip_to(self.region()) || rect.is_empty() {
            return false;
        }
        self.regions.push(rect);
        true
    }

    /// Pops the current scissor region. Returns false at the root.
    pub fn pop_region(&mut self) -> bool {
        if self.regions.len() <= 1 {
            return false;
        }
        self.regions.pop();
        true
    }
}

/// The integer size a sub-pixel jitter moves the projection: a jitter of
/// `j` pixels is `2·j·half_extent / size` in NDC, the original's offset of
/// the frustum's edges.
pub fn jitter_offset(jitter: f32, half_extent: f32, size: u32) -> f32 {
    if size == 0 {
        return 0.0;
    }
    2.0 * half_extent * jitter / size as f32
}

/// Where a full-screen copy of `source` lands inside `target` for the given
/// region, the original's texture copy: the region scaled to the target
/// rectangle, in integer pixels.
pub fn fullscreen_quad(region: Rect, target: UVec2) -> [IVec2; 4] {
    [
        IVec2::new(region.x, region.y),
        IVec2::new(region.right(), region.y),
        IVec2::new(region.right(), region.bottom()),
        IVec2::new(region.x, region.bottom()),
    ]
    .map(|p| {
        IVec2::new(
            if target.x == 0 {
                0
            } else {
                p.x * target.x as i32 / region.width.max(1)
            },
            if target.y == 0 {
                0
            } else {
                p.y * target.y as i32 / region.height.max(1)
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_pick_their_subviews_and_scales() {
        let window = UVec2::new(1600, 900);
        for (level, subviews, scale) in [
            (0, 1, 1.0),
            (1, 2, 0.75),
            (2, 3, 2.0 / 3.0),
            (3, 4, 0.6),
            (4, 8, 0.5),
        ] {
            let level = SupersampleLevel(level);
            assert_eq!(level.subviews(), subviews);
            assert_eq!(level.render_scale(), scale);
            let size = level.render_size(window);
            assert_eq!(size.x, (1600.0 * scale) as u32);
            assert_eq!(size.y, (900.0 * scale) as u32);
            assert_eq!(level.buffer_size(window), size * 2);
        }
        // The 4× setting's jitters are the documented four.
        let jitters = SupersampleLevel(3).jitters();
        assert_eq!(jitters.len(), 4);
        for (a, b) in jitters.iter().zip([
            (-0.292_626, -0.149_945),
            (0.296_924, 0.149_994),
            (0.203_849, -0.353_78),
            (-0.208_147, 0.353_73),
        ]) {
            assert_eq!(*a, b);
        }
        for j in jitters {
            assert!(j.0.abs() <= 0.5 && j.1.abs() <= 0.5);
        }
        assert_eq!(SupersampleLevel(0).jitters(), &[(0.0, 0.0)]);
        assert_eq!(SupersampleLevel(4).jitters().len(), 8);
    }

    #[test]
    fn the_sharpen_strength_follows_the_level() {
        assert_eq!(SupersampleLevel(0).sharpen(), 0.0);
        assert!((SupersampleLevel(1).sharpen() - 0.7125).abs() < 1e-6);
        assert!((SupersampleLevel(2).sharpen() - 0.825).abs() < 1e-6);
        assert!((SupersampleLevel(3).sharpen() - 0.9375).abs() < 1e-6);
        assert!((SupersampleLevel(4).sharpen() - 1.05).abs() < 1e-6);
        assert_eq!(SupersampleLevel(9).subviews(), 8);
    }

    #[test]
    fn the_projection_half_extents_follow_the_field_of_view() {
        // A 45-degree view 1 m out is 0.4142 m tall on each side at 16:10.
        let (x, y) = frustum_half_extents(45.0, 1.6, 1.0);
        assert!((y - 0.414_214).abs() < 1e-5, "{y}");
        assert!((x - 0.662_742).abs() < 1e-5, "{x}");
        // Doubling the field of view more than doubles the extents.
        let (_, y2) = frustum_half_extents(90.0, 1.6, 1.0);
        assert!((y2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn jitter_offsets_are_ndc_shifts() {
        // Half an extent across a 200-pixel view is 0.25 of the extent, so a
        // half-pixel jitter is 0.25 of the extent in NDC.
        assert_eq!(jitter_offset(0.5, 1.0, 200), 0.005);
        assert_eq!(jitter_offset(-0.5, 1.0, 200), -0.005);
        assert_eq!(jitter_offset(0.5, 1.0, 0), 0.0);
    }

    #[test]
    fn the_rect_clip_matches_the_engine() {
        // Fully inside.
        let mut r = Rect::from_edges(10, 10, 20, 20);
        assert!(r.clip_to(Rect::from_edges(0, 0, 100, 100)));
        assert_eq!(r, Rect::from_edges(10, 10, 20, 20));
        // Larger on both sides.
        r = Rect::from_edges(0, 0, 100, 100);
        assert!(r.clip_to(Rect::from_edges(10, 20, 30, 40)));
        assert_eq!(r, Rect::from_edges(10, 20, 30, 40));
        // Touching an edge is empty, not a zero-width remainder.
        r = Rect::from_edges(0, 0, 10, 10);
        assert!(!r.clip_to(Rect::from_edges(10, 0, 20, 10)));
        assert_eq!(r, Rect::EMPTY);
        // Beyond by one pixel is empty too.
        r = Rect::from_edges(0, 0, 10, 10);
        assert!(!r.clip_to(Rect::from_edges(20, 20, 30, 30)));
        assert_eq!(r, Rect::EMPTY);
        // A partial overlap keeps the intersection.
        r = Rect::from_edges(5, 5, 25, 25);
        assert!(r.clip_to(Rect::from_edges(10, 0, 20, 15)));
        assert_eq!(r, Rect::from_edges(10, 5, 20, 15));
    }

    #[test]
    fn the_viewport_stack_is_seven_deep_and_clips() {
        let mut stack = ViewportStack::new(UVec2::new(800, 600));
        assert_eq!(stack.viewport(), Rect::from_edges(0, 0, 800, 600));
        assert!(stack.push_viewport(100, 50, 200, 100));
        assert_eq!(stack.viewport(), Rect::from_edges(100, 50, 300, 150));
        // Relative to the current viewport.
        assert!(stack.push_viewport(10, 10, 50, 50));
        assert_eq!(stack.viewport(), Rect::from_edges(110, 60, 160, 110));
        // Placed from the parent's corner and clipped by the scissor region
        // (the whole target here), not by the parent's own rectangle.
        assert!(stack.pop_viewport());
        assert!(stack.push_viewport(190, 90, 200, 100));
        assert_eq!(stack.viewport(), Rect::from_edges(290, 140, 490, 240));
        // A region clips the next viewport.
        assert!(stack.push_region(0, 0, 15, 15));
        assert!(stack.push_viewport(0, 0, 100, 100));
        assert_eq!(stack.viewport(), Rect::from_edges(290, 140, 305, 155));
        assert!(stack.pop_viewport());
        assert!(stack.pop_region());
        for _ in 0..(VIEWPORT_DEPTH - stack.depth()) {
            assert!(stack.push_viewport(0, 0, 1, 1));
        }
        assert_eq!(stack.depth(), VIEWPORT_DEPTH);
        assert!(!stack.push_viewport(0, 0, 1, 1));
        while stack.pop_viewport() {}
        assert!(!stack.pop_viewport());
        // An empty push changes nothing.
        assert!(!stack.push_viewport(800, 600, 10, 10));
        assert_eq!(stack.depth(), 1);
        // The scissor region stack is independent.
        assert!(stack.push_region(10, 10, 20, 20));
        assert_eq!(stack.region(), Rect::from_edges(10, 10, 30, 30));
        assert!(stack.pop_region());
        assert!(!stack.pop_region());
    }

    #[test]
    fn a_fullscreen_copy_scales_the_region() {
        let quad = fullscreen_quad(Rect::from_edges(0, 0, 400, 300), UVec2::new(800, 600));
        assert_eq!(
            quad,
            [
                IVec2::new(0, 0),
                IVec2::new(800, 0),
                IVec2::new(800, 600),
                IVec2::new(0, 600)
            ]
        );
    }
}
