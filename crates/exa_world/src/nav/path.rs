//! The pathfinder's look-ahead and its search.
//!
//! The look-ahead is 75 + 6·v² cm along the facing (45 + 6·v² in combat,
//! 70 cm backward while backing), v in centimetres a tick. The search is
//! the original's windowed step: eight neighbours, a rise of at most four
//! floor units and a drop of less than 21, clearance at most 180/(size + 1),
//! cost 10·Δfloor² + 5 or 7 + 2·clearance + penalty, refused above a
//! penalty of 600, heuristic 10 times the cell distance. It stops at the
//! goal cell or after 4095 nodes, and keeps corners. Evidence:
//! `docs/NAVIGATION.md` and the movement notes' pathfinder.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use bevy::math::Vec3;

use super::{Layer, NavCell, NavGrid, PASSABLE};

/// Where the look-ahead is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// Exploration, forward: 75 cm + 6·v².
    Walk,
    /// Combat: 45 cm + 6·v².
    Combat,
    /// Backing: 70 cm behind the facing.
    Back,
}

/// How far (metres) along the facing the look-ahead reaches. `speed_cm` is
/// the step along the heading, centimetres a tick. Negative is behind.
pub fn look_ahead(speed_cm: f32, look: Look) -> f32 {
    let v = speed_cm.max(0.0);
    let ahead = 0.06 * v * v;
    match look {
        Look::Walk => 0.75 + ahead,
        Look::Combat => 0.45 + ahead,
        Look::Back => -0.70,
    }
}

/// A path on the grid: world points at cell centres, and whether the last
/// point is the goal cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<Vec3>,
    pub reached: bool,
    /// Search cost of the goal cell, including the penalty of each cell
    /// entered. Zero when the search does not start.
    pub cost: i64,
}

/// Searches `grid` from `from` to `to` for a walker of `size` class.
/// An empty path did not start (a point off the grid, or more than 253
/// cells apart).
pub fn find_path(grid: &NavGrid, from: Vec3, to: Vec3, size: u8) -> Path {
    let Some((sx, sz)) = grid.cell_at(from) else {
        return Path {
            points: Vec::new(),
            reached: false,
            cost: 0,
        };
    };
    let Some((gx, gz)) = grid.cell_at(to) else {
        return Path {
            points: Vec::new(),
            reached: false,
            cost: 0,
        };
    };
    if sx.abs_diff(gx) > 253 || sz.abs_diff(gz) > 253 {
        return Path {
            points: Vec::new(),
            reached: false,
            cost: 0,
        };
    }
    let limit = PASSABLE / (u32::from(size) + 1);
    let start = pick_layer(grid, sx, sz, from.y, limit);
    let goal = pick_layer(grid, gx, gz, to.y, limit);
    let (Some(start), Some(goal_layer)) = (start, goal) else {
        return Path {
            points: Vec::new(),
            reached: false,
            cost: 0,
        };
    };
    if (sx, sz) == (gx, gz) {
        return Path {
            points: vec![at(grid, sx, sz, start)],
            reached: true,
            cost: 0,
        };
    }
    search(grid, (sx, sz, start), (gx, gz, goal_layer), limit)
}

fn pick_layer(grid: &NavGrid, x: usize, z: usize, y: f32, limit: u32) -> Option<Layer> {
    let mut best: Option<(Layer, f32)> = None;
    for layer in Layer::BOTH {
        let cell = grid.cell(x, z, layer);
        if !usable(cell, limit) {
            continue;
        }
        let height = grid.floor_height(cell.floor);
        let off = (height - y).abs();
        if best.is_none_or(|(_, d)| off < d) {
            best = Some((layer, off));
        }
    }
    best.map(|(layer, _)| layer)
}

fn usable(cell: NavCell, limit: u32) -> bool {
    cell.passable(0) && u32::from(cell.clearance) <= limit && u32::from(cell.penalty) <= 600
}

fn at(grid: &NavGrid, x: usize, z: usize, layer: Layer) -> Vec3 {
    let centre = grid.cell_centre(x, z);
    let y = grid.floor_height(grid.cell(x, z, layer).floor);
    Vec3::new(centre.x, y, centre.y)
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Open {
    f: i64,
    g: i64,
    index: u32,
}

impl Ord for Open {
    fn cmp(&self, other: &Self) -> Ordering {
        self.f
            .cmp(&other.f)
            .then(self.g.cmp(&other.g))
            .then(self.index.cmp(&other.index))
    }
}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn search(
    grid: &NavGrid,
    start: (usize, usize, Layer),
    goal: (usize, usize, Layer),
    limit: u32,
) -> Path {
    let (w, d) = grid.size();
    let cells = w * d;
    let mut best_g = vec![i64::MAX; cells];
    let mut parent = vec![u32::MAX; cells];
    let mut came = vec![Layer::Lower; cells];
    let index = |x: usize, z: usize| (z * w + x) as u32;
    let start_i = index(start.0, start.1) as usize;
    best_g[start_i] = 0;
    came[start_i] = start.2;
    let mut open = BinaryHeap::new();
    open.push(std::cmp::Reverse(Open {
        f: heuristic(grid, start.0, start.1, start.2, goal),
        g: 0,
        index: index(start.0, start.1),
    }));
    let mut expanded = 0u32;
    let mut found = false;
    while let Some(std::cmp::Reverse(node)) = open.pop() {
        let (x, z) = ((node.index as usize) % w, (node.index as usize) / w);
        if best_g[node.index as usize] != node.g {
            continue;
        }
        if (x, z) == (goal.0, goal.1) {
            found = true;
            break;
        }
        expanded += 1;
        if expanded > 4095 {
            break;
        }
        let here = grid.cell(x, z, came[node.index as usize]);
        for (dx, dz, diagonal) in [
            (-1, 0, false),
            (1, 0, false),
            (0, -1, false),
            (0, 1, false),
            (-1, -1, true),
            (-1, 1, true),
            (1, -1, true),
            (1, 1, true),
        ] {
            let nx = x as i32 + dx;
            let nz = z as i32 + dz;
            if nx < 0 || nz < 0 || nx as usize >= w || nz as usize >= d {
                continue;
            }
            let (nx, nz) = (nx as usize, nz as usize);
            let layer = came[node.index as usize];
            let next = step_layer(grid, here, nx, nz, layer, limit);
            let Some(layer) = next else { continue };
            let cell = grid.cell(nx, nz, layer);
            let df = i64::from(cell.floor) - i64::from(here.floor);
            let step = if diagonal { 7 } else { 5 };
            let g = node.g
                + 10 * df * df
                + step
                + 2 * i64::from(cell.clearance)
                + i64::from(cell.penalty);
            let ni = index(nx, nz) as usize;
            if g >= best_g[ni] {
                continue;
            }
            best_g[ni] = g;
            parent[ni] = node.index;
            came[ni] = layer;
            let f = g.saturating_mul(1000) + heuristic(grid, nx, nz, layer, goal);
            open.push(std::cmp::Reverse(Open {
                f,
                g,
                index: index(nx, nz),
            }));
        }
    }
    if parent[index(goal.0, goal.1) as usize] == u32::MAX && !found {
        return Path {
            points: Vec::new(),
            reached: false,
            cost: 0,
        };
    }
    let mut chain = Vec::new();
    let mut cursor = index(goal.0, goal.1);
    loop {
        let (x, z) = ((cursor as usize) % w, (cursor as usize) / w);
        chain.push(at(grid, x, z, came[cursor as usize]));
        if cursor == index(start.0, start.1) {
            break;
        }
        let prev = parent[cursor as usize];
        if prev == u32::MAX {
            break;
        }
        cursor = prev;
    }
    chain.reverse();
    Path {
        points: corners(chain),
        reached: found,
        cost: best_g[index(goal.0, goal.1) as usize],
    }
}

fn step_layer(
    grid: &NavGrid,
    here: NavCell,
    x: usize,
    z: usize,
    from: Layer,
    limit: u32,
) -> Option<Layer> {
    for layer in [from, Layer::Lower, Layer::Upper] {
        let cell = grid.cell(x, z, layer);
        if !usable(cell, limit) {
            continue;
        }
        let df = i32::from(cell.floor) - i32::from(here.floor);
        if df <= 4 && df > -21 {
            return Some(layer);
        }
    }
    None
}

fn heuristic(grid: &NavGrid, x: usize, z: usize, layer: Layer, goal: (usize, usize, Layer)) -> i64 {
    let cell = grid.cell(x, z, layer);
    let there = grid.cell(goal.0, goal.1, goal.2);
    let df = f64::from(cell.floor) - f64::from(there.floor);
    let dx = x as f64 - goal.0 as f64;
    let dz = z as f64 - goal.1 as f64;
    (10.0 * (df * df + dx * dx + dz * dz).sqrt() * 1000.0) as i64
}

fn corners(points: Vec<Vec3>) -> Vec<Vec3> {
    if points.len() < 3 {
        return points;
    }
    let mut out = vec![points[0]];
    for window in points.windows(3) {
        let a = window[1] - window[0];
        let b = window[2] - window[1];
        let turn = a.x * b.z - a.z * b.x;
        if turn.abs() > 1e-4 {
            out.push(window[1]);
        }
    }
    out.push(*points.last().unwrap());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::NavGrid;

    fn flat(width: usize, depth: usize) -> NavGrid {
        let mut grid = NavGrid::empty(width, depth, 0.0);
        for z in 0..depth {
            for x in 0..width {
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 10,
                        clearance: 0,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    #[test]
    fn the_look_ahead_follows_the_facing() {
        assert!((look_ahead(0.0, Look::Walk) - 0.75).abs() < 1e-6);
        assert!((look_ahead(2.0, Look::Walk) - 0.99).abs() < 1e-6);
        assert!((look_ahead(2.0, Look::Combat) - 0.69).abs() < 1e-6);
        assert!((look_ahead(9.0, Look::Back) + 0.70).abs() < 1e-6);
    }

    #[test]
    fn a_path_goes_around_a_wall() {
        let mut grid = flat(8, 8);
        for z in 1..7 {
            grid.set_cell(4, z, Layer::Lower, NavCell::default());
        }
        let from = grid.cell_centre(1, 3);
        let to = grid.cell_centre(6, 3);
        let path = find_path(
            &grid,
            Vec3::new(from.x, 1.0, from.y),
            Vec3::new(to.x, 1.0, to.y),
            0,
        );
        assert!(path.reached, "{:?}", path.points);
        assert!(path.points.len() >= 2, "{:?}", path.points);
        for point in &path.points {
            let (x, z) = grid.cell_at(*point).unwrap();
            assert!(
                !(x == 4 && (1..7).contains(&z)),
                "the path enters the wall at {point}"
            );
        }
        let straight = (from - to).length();
        let walked = path
            .points
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum::<f32>();
        assert!(
            walked > straight,
            "the path did not leave the straight line"
        );
    }
}
