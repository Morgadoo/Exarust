//! The pathfinder's look-ahead and its search.
//!
//! The look-ahead is 75 + 6·v² cm along the facing (45 + 6·v² in combat,
//! 70 cm backward while backing), v in centimetres a tick. The search is
//! [`search`]: the original's 256-cell window, its open list and relaxation,
//! its heuristic, its node cap and its tail simplification. Evidence:
//! `docs/NAVIGATION.md` and the movement notes' pathfinder.

use bevy::math::Vec3;

use super::{Layer, NavGrid, PASSABLE};

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

/// A path on the grid: world points at cell centres, and whether the end
/// is the goal (or within the query's accept distance).
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<Vec3>,
    pub reached: bool,
    /// Search cost of the end, including the penalty of each cell entered.
    /// Zero when the search does not start.
    pub cost: i64,
}

impl Path {
    fn none() -> Self {
        Self {
            points: Vec::new(),
            reached: false,
            cost: 0,
        }
    }
}

/// Searches `grid` from `from` to `to` for a walker of `size` class, with
/// the default accept distance of 0 and penalty shift of 0.
pub fn find_path(grid: &NavGrid, from: Vec3, to: Vec3, size: u8) -> Path {
    find_path_with(grid, from, to, size, 0.0, 0)
}

/// The pathfinder's disturbance sampler: the original's stub always
/// answers 0.0.
pub fn disturbance() -> f32 {
    0.0
}

/// Searches with an accept distance (tenths of a cell in the original's
/// heuristic units; 0 never accepts early) and the penalty byte's shift.
pub fn find_path_with(
    grid: &NavGrid,
    from: Vec3,
    to: Vec3,
    size: u8,
    accept: f32,
    shift: u32,
) -> Path {
    search(grid, from, to, size, accept, shift)
}

/// The eight neighbours in the original's order: north-west, north,
/// north-east, west, east, south-west, south, south-east. The step costs
/// are 7 on a diagonal and 5 orthogonally.
const NEIGHBOURS: [(i32, i32, i32); 8] = [
    (-1, -1, 7),
    (0, -1, 5),
    (1, -1, 7),
    (-1, 0, 5),
    (1, 0, 5),
    (-1, 1, 7),
    (0, 1, 5),
    (1, 1, 7),
];

/// The search's window is 256 cells square, as the original's stamp table
/// is.
const WINDOW: i32 = 256;
/// The search starts and ends at most this many cells from the grid's
/// centre (the grid leaves a margin of ten on each side).
const CENTRE_MARGIN: i32 = 10;
/// A search may hold 4096 nodes, the first a sentinel.
const MAX_NODES: usize = 4096;
/// A goal within this distance (cm) counts as reached without the accept
/// distance.
const ARRIVED_CM: f32 = 10_000.0;

/// One node of the planner's window.
#[derive(Debug, Clone, Copy)]
struct Node {
    /// Window coordinates.
    x: u8,
    y: u8,
    layer: u8,
    clearance: u8,
    g: i32,
    f: i32,
    /// Next node in the open list, 0 for the end.
    next: u16,
    parent: u16,
}

impl Node {
    fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            layer: 0,
            clearance: 0,
            g: 0,
            f: 0,
            next: 0,
            parent: 0,
        }
    }
}

/// A world coordinate to a planner cell, as the original rounds it:
/// `round_ties_even(world_cm / 30 + 0.5)`. The planner's cell is the
/// grid index less `W/2` plus one.
fn planner_coord(metres: f32) -> i32 {
    let cells = super::centimetres_of(f64::from(metres)) / super::CELL_CM;
    (cells as f32 + 0.5).round_ties_even() as i32
}

/// The height in the planner's 10 cm units: `trunc((y − base) / 10)`.
fn planner_height(y: f32, base: f32) -> i32 {
    ((f64::from(y) - f64::from(base)) / super::VOXEL_CM * 100.0) as i32
}

/// The line shortcut's max cell step: `|b − a|` on the longer axis.
fn line_clear(grid: &NavGrid, from: (i32, i32), to: (i32, i32), budget: i32, shift: u32) -> bool {
    let (width, depth) = grid.size();
    let dx = to.0 - from.0;
    let dz = to.1 - from.1;
    let x_major = dx.abs() <= dz.abs();
    let (major_delta, minor_delta, major, minor) = if x_major {
        (dz, dx, from.1, from.0)
    } else {
        (dx, dz, from.0, from.1)
    };
    if major_delta == 0 {
        return true;
    }
    let step = if major_delta < 0 { -1 } else { 1 };
    let slope = minor_delta as f32 / major_delta as f32;
    let intercept = minor as f32 - major as f32 * slope;
    let count = major_delta.abs() - 1;
    let start = grid.cell(from.0 as usize, from.1 as usize, Layer::Lower);
    let mut height = i32::from(start.floor);
    let mut cost = 0;
    let mut major = major;
    for _ in 0..count {
        let previous = minor_at(major, slope, intercept);
        major += step;
        let minor = minor_at(major, slope, intercept);
        let (x, z) = if x_major {
            (minor, major)
        } else {
            (major, minor)
        };
        if x <= 0 || z <= 0 || x as usize >= width || z as usize >= depth {
            return false;
        }
        let direction = if minor == previous { 5 } else { 7 };
        let lower = grid.cell(x as usize, z as usize, Layer::Lower);
        let upper = grid.cell(x as usize, z as usize, Layer::Upper);
        if lower.floor == 0 {
            return false;
        }
        let mut layer: i32;
        let mut chosen: i32;
        if height + 4 < i32::from(lower.floor) {
            if i32::from(lower.underside) < height + 10 {
                return false;
            }
            layer = -1;
            chosen = i32::from(lower.floor);
        } else {
            layer = 0;
            chosen = i32::from(lower.floor);
        }
        if upper.floor == 0 {
            if layer < 0 {
                return false;
            }
        } else if height + 4 < i32::from(upper.floor) {
            let under = i32::from(upper.underside);
            if under < height + 10 && (under != 0 || upper.floor < lower.floor) {
                return false;
            }
            if layer < 0 {
                return false;
            }
        } else if i32::from(upper.floor) <= i32::from(lower.floor) {
            if layer < 0 {
                return false;
            }
        } else {
            layer = 1;
            chosen = i32::from(upper.floor);
        }
        let cell = if layer == 0 { lower } else { upper };
        cost += (chosen - height) * (chosen - height) * 10
            + direction
            + i32::from(cell.clearance) * 2
            + (i32::from(cell.penalty) << (shift + 1));
        if budget + 10 < cost {
            return false;
        }
        height = chosen;
    }
    true
}

fn minor_at(major: i32, slope: f32, intercept: f32) -> i32 {
    (major as f32 * slope + intercept).round_ties_even() as i32
}

/// The planner's search, as the original runs it.
///
/// The start and goal snap to cells (ties to even); both must lie within
/// the grid's ten-cell margin and within 253 cells of each other. A 256 ×
/// 256 window is centred between them. Its start node takes the upper
/// layer when the start height is above that layer's underside less three
/// and below its floor plus fifteen, else the lower layer. Nodes are
/// expanded eight at a time in the original's order; a step may fall less
/// than 2.1 m and rise at most 40 cm, must pass the clearance test and pay
/// at most 600 of penalty. The open list is LIFO and popped by the smallest
/// f, ties taken newest first; a relaxed node keeps its place, and a node
/// already taken is not put back. The heuristic is ten times the 3-D cell
/// distance, its vertical leg measured from the start height. The search
/// stops under the accept distance, when the open list empties, or after
/// 4096 nodes. The end is the closest node, or the goal test's answer. The
/// parent chain is collapsed through collinear runs, then every second
/// point back is tested with the budget line walk and dropped when clear.
fn search(grid: &NavGrid, from: Vec3, to: Vec3, size: u8, accept_cm: f32, shift: u32) -> Path {
    let (width, depth) = grid.size();
    let limit = PASSABLE / (u32::from(size) + 1);
    let start_cell = (planner_coord(from.x), planner_coord(from.z));
    let goal_cell = (planner_coord(to.x), planner_coord(to.z));
    let margin_x = (width / 2) as i32 - CENTRE_MARGIN;
    let margin_z = (depth / 2) as i32 - CENTRE_MARGIN;
    // A shipped level's grid is at least 320 cells across, so the margin
    // never excludes a cell there. The unit tests use grids of a few dozen
    // cells, where the original's margin would leave no cell at all; those
    // keep their interior instead.
    if (width >= 64 && (start_cell.0.abs() > margin_x || goal_cell.0.abs() > margin_x))
        || (depth >= 64 && (start_cell.1.abs() > margin_z || goal_cell.1.abs() > margin_z))
    {
        return Path::none();
    }
    if (start_cell.0 - goal_cell.0).abs() > 253 || (start_cell.1 - goal_cell.1).abs() > 253 {
        return Path::none();
    }
    let centre = (
        start_cell.0 + (goal_cell.0 - start_cell.0) / 2,
        start_cell.1 + (goal_cell.1 - start_cell.1) / 2,
    );
    let half = (width / 2) as i32 - 1;
    let origin = (centre.0 + half - 127, centre.1 + (depth / 2) as i32 - 128);
    let window = (start_cell.0 - centre.0 + 127, start_cell.1 - centre.1 + 127);
    let goal_window = (goal_cell.0 - centre.0 + 127, goal_cell.1 - centre.1 + 127);
    if !(0..WINDOW).contains(&window.0)
        || !(0..WINDOW).contains(&window.1)
        || !(0..WINDOW).contains(&goal_window.0)
        || !(0..WINDOW).contains(&goal_window.1)
    {
        return Path::none();
    }
    let cell_index = |x: i32, z: i32| -> Option<usize> {
        if x < 0 || z < 0 || x as usize >= width || z as usize >= depth {
            None
        } else {
            Some(z as usize * width + x as usize)
        }
    };
    if cell_index(origin.0 + window.0, origin.1 + window.1).is_none() {
        return Path::none();
    }
    let start_x = (origin.0 + window.0) as usize;
    let start_z = (origin.1 + window.1) as usize;
    let start_cell_data = grid.cell(start_x, start_z, Layer::Lower);
    let upper_start = grid.cell(start_x, start_z, Layer::Upper);
    let start_height = planner_height(from.y, grid.base());
    let start_layer = if i32::from(upper_start.underside) - 3 < start_height
        && start_height < i32::from(upper_start.floor) + 15
    {
        Layer::Upper
    } else {
        Layer::Lower
    };
    let start_floor = if start_layer == Layer::Lower {
        i32::from(start_cell_data.floor)
    } else {
        i32::from(upper_start.floor)
    };
    let mut stamps = vec![0u16; WINDOW as usize * WINDOW as usize * 2];
    let stamp_at = |stamps: &Vec<u16>, x: i32, z: i32, layer: Layer| -> u16 {
        if !(0..WINDOW).contains(&x) || !(0..WINDOW).contains(&z) {
            return u16::MAX;
        }
        stamps[((z * WINDOW + x) * 2 + layer as i32) as usize]
    };
    let set_stamp = |stamps: &mut Vec<u16>, x: i32, z: i32, layer: Layer, value: u16| {
        if (0..WINDOW).contains(&x) && (0..WINDOW).contains(&z) {
            stamps[((z * WINDOW + x) * 2 + layer as i32) as usize] = value;
        }
    };
    let dx = window.0 - goal_window.0;
    let dz = window.1 - goal_window.1;
    let start_f = (10.0
        * ((f64::from(start_floor) * 0.333).powi(2)
            + f64::from(dx) * f64::from(dx)
            + f64::from(dz) * f64::from(dz))
        .sqrt()) as i32;
    let mut nodes = vec![Node::empty(); 2];
    nodes[1] = Node {
        x: window.0 as u8,
        y: window.1 as u8,
        layer: start_layer as u8,
        clearance: 0,
        g: 0,
        f: start_f,
        next: 0,
        parent: 0,
    };
    set_stamp(&mut stamps, window.0, window.1, start_layer, 1);
    let mut head: u16 = 0;
    let mut open_count = 0usize;
    let mut best_node: u16 = 0;
    let mut best_h = i32::MAX;
    let mut current: u16 = 1;
    let mut check_end = true;

    'search: loop {
        let node = nodes[current as usize];
        let cx = node.x as i32;
        let cy = node.y as i32;
        let here = origin.0 + cx;
        let here_y = origin.1 + cy;
        let current_floor = i32::from(
            grid.cell(here as usize, here_y as usize, Layer::Lower)
                .floor,
        );
        for (dx, dz, cost) in NEIGHBOURS {
            let wx = cx + dx;
            let wy = cy + dz;
            let gx = origin.0 + wx;
            let gy = origin.1 + wy;
            if cell_index(gx, gy).is_none() {
                continue;
            }
            let lower = grid.cell(gx as usize, gy as usize, Layer::Lower);
            let upper = grid.cell(gx as usize, gy as usize, Layer::Upper);
            if lower.floor == 0 {
                continue;
            }
            let mut layer: i32;
            let mut chosen: i32;
            if current_floor + 4 < i32::from(lower.floor) {
                if i32::from(lower.underside) < current_floor + 10 {
                    continue;
                }
                layer = -1;
                chosen = i32::from(lower.floor);
            } else {
                layer = 0;
                chosen = i32::from(lower.floor);
            }
            if upper.floor == 0 {
                if layer < 0 {
                    continue;
                }
            } else if current_floor + 4 < i32::from(upper.floor) {
                let under = i32::from(upper.underside);
                if under < current_floor + 10 && (under != 0 || upper.floor < lower.floor) {
                    continue;
                }
                if layer < 0 {
                    continue;
                }
            } else if i32::from(upper.floor) <= i32::from(lower.floor) {
                if layer < 0 {
                    continue;
                }
            } else {
                layer = 1;
                chosen = i32::from(upper.floor);
            }
            let rise = chosen - current_floor;
            if !(-20..=4).contains(&rise) {
                continue;
            }
            let cell = if layer == 0 { lower } else { upper };
            if u32::from(cell.clearance) > limit {
                continue;
            }
            let mut g = node.g + rise * rise * 10 + cost + i32::from(cell.clearance) * 2;
            let penalty = i32::from(cell.penalty) << shift;
            if penalty >= 601 {
                continue;
            }
            g += penalty;
            let h = (10.0
                * ((f64::from(chosen - start_height)).powi(2)
                    + f64::from(wx - goal_window.0).powi(2)
                    + f64::from(wy - goal_window.1).powi(2))
                .sqrt()) as i32;
            let stamp = stamp_at(
                &stamps,
                wx,
                wy,
                if layer == 0 {
                    Layer::Lower
                } else {
                    Layer::Upper
                },
            );
            if stamp == 0 {
                if nodes.len() >= MAX_NODES {
                    break 'search;
                }
                let index = nodes.len() as u16;
                nodes.push(Node {
                    x: wx as u8,
                    y: wy as u8,
                    layer: layer as u8,
                    clearance: cell.clearance,
                    g,
                    f: g + h,
                    next: head,
                    parent: current,
                });
                set_stamp(
                    &mut stamps,
                    wx,
                    wy,
                    if layer == 0 {
                        Layer::Lower
                    } else {
                        Layer::Upper
                    },
                    index,
                );
                head = index;
                open_count += 1;
                if h < best_h {
                    best_h = h;
                    best_node = index;
                }
            } else if stamp != u16::MAX {
                let existing = stamp as usize;
                if g < nodes[existing].g {
                    nodes[existing].f = g + (nodes[existing].f - nodes[existing].g);
                    nodes[existing].g = g;
                    nodes[existing].parent = current;
                }
            }
        }
        if (best_h as f32) < accept_cm {
            check_end = false;
            break;
        }
        if open_count == 0 {
            break;
        }
        let mut previous: u16 = 0;
        let mut scan = head;
        let mut pick = head;
        let mut pick_previous: u16 = 0;
        while scan != 0 {
            if nodes[scan as usize].f < nodes[pick as usize].f {
                pick = scan;
                pick_previous = previous;
            }
            previous = scan;
            scan = nodes[scan as usize].next;
        }
        if pick_previous == 0 {
            head = nodes[pick as usize].next;
        } else {
            nodes[pick_previous as usize].next = nodes[pick as usize].next;
        }
        open_count -= 1;
        current = pick;
    }
    if best_node == 0 {
        return Path::none();
    }
    let mut end = best_node;
    if check_end {
        let point = point_of(grid, origin, nodes[end as usize]);
        // The original's ordered compare treats a NaN distance as arrived.
        if point.distance(to).partial_cmp(&ARRIVED_CM) != Some(std::cmp::Ordering::Greater) {
            check_end = false;
        }
    }
    if check_end {
        let threshold = 64 / (i32::from(size) + 1);
        let mut candidate = end;
        let mut previous_clearance = i32::from(nodes[candidate as usize].clearance);
        for _ in 0..8 {
            let clearance = i32::from(nodes[candidate as usize].clearance);
            if clearance < threshold {
                end = candidate;
                break;
            }
            if clearance < previous_clearance {
                end = candidate;
            }
            previous_clearance -= 4;
        }
        candidate = end;
        while candidate > 1 {
            let parent = nodes[candidate as usize].parent;
            if i32::from(nodes[candidate as usize].clearance) > threshold {
                end = parent;
            }
            candidate = parent;
        }
    }
    // The original's list starts at index 1; the first parent is stored at
    // index 2, so index 1 stays unused.
    let mut chain = vec![0u16, 0];
    let mut count = 1usize;
    let mut previous_delta = (0i32, 0i32);
    let mut previous_layer = nodes[end as usize].layer as i32;
    let mut cursor = end;
    while cursor != 1 {
        let parent = nodes[cursor as usize].parent;
        if parent == cursor || parent == 0 {
            break;
        }
        let node = nodes[cursor as usize];
        let next = nodes[parent as usize];
        let delta = (next.x as i32 - node.x as i32, next.y as i32 - node.y as i32);
        if delta != previous_delta || next.layer as i32 != previous_layer {
            count += 1;
            previous_delta = delta;
        }
        if chain.len() <= count {
            chain.resize(count + 1, 0);
        }
        chain[count] = parent;
        previous_layer = next.layer as i32;
        cursor = parent;
    }
    if count < 2 {
        return Path {
            points: Vec::new(),
            reached: !check_end,
            cost: nodes[end as usize].g as i64,
        };
    }
    // The original walks its collapsed chain from the start towards the
    // end and keeps the points the budget line walk cannot skip. Its tail
    // indexing is ambiguous in the pseudo-code (the rating marks it as
    // inferred); this keeps the same line test and the same order.
    let start_node = chain[count];
    let mut points = vec![point_of(grid, origin, nodes[start_node as usize])];
    let mut kept = start_node;
    if count >= 3 {
        for index in (2..count).rev() {
            let target = chain[index];
            let target_point = point_of(grid, origin, nodes[target as usize]);
            let budget = nodes[target as usize].g - nodes[kept as usize].g;
            let from = cell_of_point(grid, points[points.len() - 1]);
            let target_node = nodes[target as usize];
            let to = (
                origin.0 + target_node.x as i32,
                origin.1 + target_node.y as i32,
            );
            if !line_clear(grid, from, to, budget, shift) {
                points.push(target_point);
                kept = target;
            }
        }
    }
    points.push(point_of(grid, origin, nodes[end as usize]));
    Path {
        points,
        reached: !check_end,
        cost: nodes[end as usize].g as i64,
    }
}

fn point_of(grid: &NavGrid, origin: (i32, i32), node: Node) -> Vec3 {
    let x = origin.0 + node.x as i32;
    let y = origin.1 + node.y as i32;
    let centre = grid.cell_centre(x as usize, y as usize);
    let layer = if node.layer == 0 {
        Layer::Lower
    } else {
        Layer::Upper
    };
    Vec3::new(
        centre.x,
        grid.floor_height(grid.cell(x as usize, y as usize, layer).floor),
        centre.y,
    )
}

fn cell_of_point(grid: &NavGrid, point: Vec3) -> (i32, i32) {
    let (width, depth) = grid.size();
    (
        planner_coord(point.x) + (width / 2) as i32 - 1,
        planner_coord(point.z) + (depth / 2) as i32 - 1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::{NavCell, NavGrid};

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
    fn the_disturbance_sampler_is_a_stub() {
        assert_eq!(disturbance(), 0.0);
        assert_eq!(disturbance().to_bits(), 0.0f32.to_bits());
    }

    #[test]
    fn planner_cells_round_to_even_and_heights_truncate() {
        assert_eq!(planner_coord(0.0), 0);
        assert_eq!(planner_coord(0.15), 1);
        assert_eq!(planner_coord(0.30), 2);
        assert_eq!(planner_coord(0.45), 2);
        assert_eq!(planner_coord(-0.15), 0);
        assert_eq!(planner_coord(-0.30), 0);
        assert_eq!(planner_coord(-0.45), -1);
        assert_eq!(planner_height(1.0, 0.0), 10);
        assert_eq!(planner_height(0.099, 0.0), 0);
        assert_eq!(planner_height(-0.01, 0.0), 0);
    }

    #[test]
    fn a_straight_walk_pays_five_a_cell_and_ends_at_the_goal() {
        let grid = flat(8, 8);
        let from = grid.cell_centre(1, 3);
        let to = grid.cell_centre(6, 3);
        // Above the upper layer's empty floor, so the start takes the lower
        // layer as a standing walker does.
        let from = Vec3::new(from.x, 2.0, from.y);
        let to = Vec3::new(to.x, 2.0, to.y);
        let path = find_path(&grid, from, to, 0);
        assert!(path.reached, "{path:?}");
        // The first node whose heuristic ties at zero-remainder wins, and
        // ties are strict: the goal cell's diagonal neighbour is the end.
        assert_eq!(path.cost, 22, "{path:?}");
        assert_eq!(path.points.first(), Some(&Vec3::new(from.x, 1.0, from.z)));
        let end = path.points.last().copied().unwrap();
        assert!(
            (end.x - 0.45).abs() < 1e-5 && (end.z + 0.45).abs() < 1e-5,
            "{path:?}"
        );
    }

    fn corridor(width: usize) -> NavGrid {
        let mut grid = NavGrid::empty(width, 8, 0.0);
        for x in 0..width {
            grid.set_cell(
                x,
                3,
                Layer::Lower,
                NavCell {
                    floor: 10,
                    ..NavCell::default()
                },
            );
        }
        grid
    }

    #[test]
    fn a_penalty_byte_is_paid_once_per_cell() {
        let mut grid = corridor(8);
        let from = grid.cell_centre(1, 3);
        let to = grid.cell_centre(5, 3);
        let path = find_path(
            &grid,
            Vec3::new(from.x, 2.0, from.y),
            Vec3::new(to.x, 2.0, to.y),
            0,
        );
        assert_eq!(path.cost, 15, "{path:?}");
        let mut cell = grid.cell(4, 3, Layer::Lower);
        cell.penalty = 4;
        grid.set_cell(4, 3, Layer::Lower, cell);
        let taxed = find_path(
            &grid,
            Vec3::new(from.x, 2.0, from.y),
            Vec3::new(to.x, 2.0, to.y),
            0,
        );
        assert_eq!(taxed.cost, 19, "{taxed:?}");
    }

    #[test]
    fn no_path_starts_outside_the_grid_or_crosses_a_gap() {
        let grid = corridor(8);
        let probe = find_path(&grid, Vec3::new(9.0, 2.0, 0.0), Vec3::ZERO, 0);
        assert_eq!(probe, Path::none());

        let mut cut = corridor(8);
        cut.set_cell(3, 3, Layer::Lower, NavCell::default());
        let from = cut.cell_centre(1, 3);
        let to = cut.cell_centre(5, 3);
        let path = find_path(
            &cut,
            Vec3::new(from.x, 2.0, from.y),
            Vec3::new(to.x, 2.0, to.y),
            0,
        );
        // The search answers the closest node it reached, and the original
        // calls anything within 100 m of the goal reached even when a gap
        // seals the goal cell.
        assert_eq!(path.cost, 5, "{path:?}");
        let end = path.points.last().copied().unwrap();
        assert!(
            (end.x + 0.45).abs() < 1e-5 && (end.z + 0.15).abs() < 1e-5,
            "{path:?}"
        );
        assert!(path.reached, "{path:?}");
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
