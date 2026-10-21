//! The pathfinder the game queries: the level's grid once it is built, and
//! the abstract 64 × 64 fallback a level without one installs.
//!
//! The original keeps one global pathfinder. Each level load builds a
//! `TPathFinder` from the level's scene (reusing the one already there when
//! it is of the right class) and points the global at it; the unit's init
//! also creates a 64 × 64 abstract pathfinder, and the global falls back to
//! it while there is no level pathfinder. The abstract pathfinder answers
//! every query with the neutral value the game treats as "nothing here":
//! no obstacle, no clamped line, no snapping, no blocked point, no reach, a
//! zeroed value, a height of 0, the point passed through unchanged, and an
//! empty call. That is why a level without a grid never blocks the player's
//! look-ahead.
//!
//! Exarust builds the grid asynchronously ([`super::view`]) and keeps it as
//! the [`NavGrid`] resource. [`InstalledPathfinder`] records which object the
//! game would have installed: the level's grid, or the fallback while the
//! grid is being built or when the level has none. The queries that take an
//! `Option<&NavGrid>` answer through it. Evidence: the original's unit glue
//! (create or reuse the level's pathfinder, install it or the fallback,
//! free the fallback at shutdown) and the abstract class's 15 stubs.

use bevy::math::Vec3;
use bevy::prelude::*;

use super::path::{self, Path};
use super::query::{self, LineHit};
use super::walkable;
use super::{NavGrid, find_path};

/// Cells across the fallback pathfinder.
pub const FALLBACK_SIZE: u32 = 64;

/// The pathfinder interface's neutral answers: the original's abstract
/// class, which every query answers as if there were nothing there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbstractPathFinder {
    width: u32,
    depth: u32,
    owner: Option<u64>,
}

impl Default for AbstractPathFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl AbstractPathFinder {
    /// The 64 × 64 fallback.
    pub fn new() -> Self {
        Self::with_size(FALLBACK_SIZE, FALLBACK_SIZE)
    }

    /// A fallback of the given size, as a level's grid would have (the
    /// original only ever makes the 64 × 64 one).
    pub fn with_size(width: u32, depth: u32) -> Self {
        Self {
            width,
            depth,
            owner: None,
        }
    }

    /// Attaches the pathfinder to its owner (the scene or level it belongs
    /// to) and returns it.
    pub fn attach(&mut self, owner: u64) -> &mut Self {
        self.owner = Some(owner);
        self
    }

    /// The owner an attach stored.
    pub fn owner(&self) -> Option<u64> {
        self.owner
    }

    /// The cells across and along.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.depth)
    }

    /// The scene view, which the abstract class answers with itself.
    pub fn scene(&self) -> &Self {
        self
    }

    /// The grid view, which the abstract class answers with itself.
    pub fn grid(&self) -> &Self {
        self
    }

    /// The planner view, which the abstract class answers with itself.
    pub fn planner(&self) -> &Self {
        self
    }

    /// The word the two-point query answers: the third word of the second
    /// point, whatever the first. It is the abstract class's only point
    /// query that returns a word rather than a flag [guess: what the caller
    /// reads it as; no engine caller was resolved].
    pub fn end_word(&self, _start: Vec3, end: Vec3) -> u32 {
        end.z.to_bits()
    }

    /// The line walk: nothing blocks, and the line keeps its asked end.
    pub fn line_walk(&self, _start: Vec3, end: Vec3) -> LineHit {
        LineHit {
            blocked: false,
            end,
        }
    }

    /// Whether the point is blocked: never.
    pub fn blocked(&self, _point: Vec3) -> bool {
        false
    }

    /// Whether the point can be reached: never.
    pub fn reaches(&self, _point: Vec3) -> bool {
        false
    }

    /// Snaps a point onto walkable ground: the abstract class never does,
    /// and leaves the point as it was.
    pub fn snap(&self, _point: &mut Vec3) -> bool {
        false
    }

    /// Clamps a point back from an obstacle: unchanged.
    pub fn clamp_point(&self, point: Vec3) -> Vec3 {
        point
    }

    /// The byte an update hands the pathfinder, returned as it came (the
    /// abstract class stores nothing).
    pub fn notice(&mut self, value: u8) -> u8 {
        value
    }

    /// The zeroed eight-byte value (a `double` 0.0 by its bits).
    pub fn zero(&self) -> f64 {
        0.0
    }

    /// The height-like scalar the abstract class answers 0.0 for.
    pub fn height(&self) -> f32 {
        0.0
    }

    /// A call that does nothing.
    pub fn no_op(&self) {}

    /// The path planner's answer: no path (no points, not reached, cost 0).
    pub fn plan(&self) -> Path {
        Path {
            points: Vec::new(),
            reached: false,
            cost: 0,
        }
    }
}

/// Which object the game's one pathfinder is: the level's grid or the
/// abstract fallback.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstalledPathfinder {
    /// No level grid (none yet, or the level has none): the fallback.
    #[default]
    Fallback,
    /// The level's grid, the `NavGrid` resource.
    Level,
}

impl InstalledPathfinder {
    /// The object a level load installs: its pathfinder (the grid resource
    /// once the build finishes).
    pub fn install_level() -> Self {
        Self::Level
    }

    /// The object the unit init creates, and the one a level without a
    /// pathfinder leaves installed.
    pub fn install_fallback() -> Self {
        Self::Fallback
    }

    /// The level's grid, when the level's pathfinder is installed and the
    /// grid is present.
    pub fn level<'a>(&self, grid: Option<&'a NavGrid>) -> Option<&'a NavGrid> {
        match self {
            Self::Level => grid,
            Self::Fallback => None,
        }
    }

    /// The cells across and along of the installed pathfinder (the grid's,
    /// or the fallback's 64 × 64).
    pub fn size(&self, grid: Option<&NavGrid>) -> (u32, u32) {
        match self.level(grid) {
            Some(grid) => {
                let (w, d) = grid.size();
                (w as u32, d as u32)
            }
            None => (FALLBACK_SIZE, FALLBACK_SIZE),
        }
    }

    /// A point's clearance and penalty: zero with the fallback.
    pub fn cost(&self, grid: Option<&NavGrid>, point: Vec3) -> (f32, f32) {
        self.level(grid)
            .map_or((0.0, 0.0), |grid| walkable::cost_at(grid, point))
    }

    /// Whether a line meets an obstacle: nothing with the fallback.
    pub fn obstacle(&self, grid: Option<&NavGrid>, start: Vec3, end: Vec3) -> bool {
        self.level(grid)
            .is_some_and(|grid| query::obstacle(grid, start, end))
    }

    /// The line walk: the grid's, or the line unchanged with the fallback.
    pub fn line_walk(&self, grid: Option<&NavGrid>, start: Vec3, end: Vec3) -> LineHit {
        match self.level(grid) {
            Some(grid) => query::line_walk(grid, start, end),
            None => LineHit {
                blocked: false,
                end,
            },
        }
    }

    /// The nearest walkable point: the same point with the fallback.
    pub fn nearest_walkable(&self, grid: Option<&NavGrid>, point: Vec3) -> Vec3 {
        self.level(grid)
            .map_or(point, |grid| walkable::nearest_walkable(grid, point))
    }

    /// The disturbance the planner samples: zero with the fallback (the
    /// original's real pathfinder answers zero too).
    pub fn disturbance(&self) -> f32 {
        path::disturbance()
    }

    /// The path from `from` to `to`: no path with the fallback.
    pub fn find_path(
        &self,
        grid: Option<&NavGrid>,
        from: Vec3,
        to: Vec3,
        size: u8,
    ) -> Option<Path> {
        self.level(grid).map(|grid| find_path(grid, from, to, size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{Layer, NavCell};

    #[test]
    fn the_fallback_answers_every_query_with_nothing() {
        let mut f = AbstractPathFinder::new();
        assert_eq!(f.size(), (64, 64));
        assert_eq!(f.owner(), None);
        assert_eq!(f.attach(7).owner(), Some(7));
        assert_eq!(f.attach(7).size(), (64, 64));
        assert!(std::ptr::eq(f.scene(), &f));
        assert!(std::ptr::eq(f.grid(), &f));
        assert!(std::ptr::eq(f.planner(), &f));
        let point = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(f.end_word(Vec3::X, point), 3.0f32.to_bits());
        let hit = f.line_walk(Vec3::ZERO, point);
        assert!(!hit.blocked);
        assert_eq!(hit.end, point);
        assert!(!f.blocked(point));
        assert!(!f.reaches(point));
        let mut snapped = point;
        assert!(!f.snap(&mut snapped));
        assert_eq!(snapped, point);
        assert_eq!(f.clamp_point(point), point);
        assert_eq!(f.notice(9), 9);
        assert_eq!(f.zero().to_bits(), 0);
        assert_eq!(f.height(), 0.0);
        f.no_op();
        let path = f.plan();
        assert!(path.points.is_empty() && !path.reached && path.cost == 0);
    }

    #[test]
    fn the_installed_pathfinder_is_the_grid_or_the_fallback() {
        let mut grid = NavGrid::empty(4, 3, 0.0);
        grid.set_cell(0, 0, Layer::Lower, NavCell::default());
        assert_eq!(AbstractPathFinder::with_size(2, 5).size(), (2, 5));

        let installed = InstalledPathfinder::default();
        assert_eq!(installed, InstalledPathfinder::Fallback);
        assert_eq!(installed.size(None), (64, 64));
        assert_eq!(installed.size(Some(&grid)), (64, 64));
        assert_eq!(installed.cost(Some(&grid), Vec3::ZERO), (0.0, 0.0));
        assert!(!installed.obstacle(Some(&grid), Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0)));
        let hit = installed.line_walk(Some(&grid), Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0));
        assert!(!hit.blocked);
        assert_eq!(hit.end, Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(
            installed.nearest_walkable(Some(&grid), Vec3::new(5.0, 1.0, 5.0)),
            Vec3::new(5.0, 1.0, 5.0)
        );
        assert!(
            installed
                .find_path(Some(&grid), Vec3::ZERO, Vec3::ZERO, 0)
                .is_none()
        );

        let level = InstalledPathfinder::install_level();
        assert_eq!(level, InstalledPathfinder::Level);
        assert_eq!(level.size(Some(&grid)), (4, 3));
        assert!(level.level(Some(&grid)).is_some());
        assert!(
            level
                .find_path(Some(&grid), Vec3::ZERO, Vec3::ZERO, 0)
                .is_some()
        );
        // A level installed before its grid exists still answers nothing.
        assert_eq!(level.size(None), (64, 64));
        assert!(level.find_path(None, Vec3::ZERO, Vec3::ZERO, 0).is_none());
        assert_eq!(
            InstalledPathfinder::install_fallback().size(Some(&grid)),
            (64, 64)
        );
    }
}
