//! The pathfinder's look-ahead and its planner.
//!
//! The look-ahead is 75 + 6·v² cm along the facing (45 + 6·v² in combat,
//! 70 cm backward while backing), v in centimetres a tick.
//!
//! The planner ([`find_path_with`]) is the original's windowed A\*, read
//! instruction by instruction:
//!
//! - **Cells.** The start and the goal become planner cells by rounding
//!   `cm · (1/30) + ½` ties to even (in single precision); a planner cell is
//!   the grid index less `W/2` plus one. Both must lie within the grid's
//!   half size less ten and within 253 cells of each other; a 256 × 256
//!   window is centred between them (the integer half of their offset, cut
//!   toward zero) and its border is never entered.
//! - **Heights.** The start's and the goal's heights are
//!   `round((y − base) · 0.1)` (centimetres, the product in double, ties to
//!   even). The start takes the upper layer when its height lies above that
//!   layer's underside less three and below its floor plus fifteen.
//! - **Steps.** From a node's floor (on its own layer) to the eight
//!   neighbours in the original's order. A cell without a lower floor is
//!   refused. A lower floor more than four above refuses the cell unless
//!   ten lie under its slab, and then the upper floor must take the step.
//!   An upper floor more than four above refuses the cell when less than
//!   ten lie under its slab (an upper slab with nothing measured under it
//!   and its floor not below the lower one passes); within four and higher
//!   than the lower floor it is the one entered. The rise must be −20 to
//!   +4, and the entered layer's clearance at most `180/(size + 1)`.
//! - **Cost.** `10·Δ² + 5 or 7 + 2·clearance + (penalty << shift)`; a
//!   shifted penalty over 600 refuses the cell.
//! - **Order.** Each new node goes in front of a LIFO list; the node of
//!   least total (compared unsigned) is taken next, of equals the newest. A
//!   node met again more cheaply (its cost compared unsigned) takes the new
//!   parent and cost, keeping its place; a taken node is never put back.
//!   The heuristic of a new node is `round(10·√(Δx² + Δz² + Δh²))`, `Δh`
//!   its floor less the goal's height; the closest new node is kept.
//! - **End.** The search stops when the closest node's heuristic is under
//!   the accept distance (the goal is reached), when nothing waits, or when
//!   a 4096th node would be made (the accept test still runs). Without a
//!   closest node there is no path. When the search did not reach the goal,
//!   the closest node's cell centre on its floor counts as reached within
//!   100 cm of the goal (in 3-D, squared in single precision); otherwise the
//!   path ends at the parent of the rough node (clearance over
//!   `64/(size + 1)`) nearest the start.
//! - **Corners.** The parent chain from the end keeps the last node of each
//!   run of one direction on one layer. From the start, each corner after
//!   the next is tested with the budget line walk back to the last point
//!   kept: when the line is blocked, the corner before it is kept. The line
//!   walk ([`line_blocked`]) holds the far corner's floor, steps the longer
//!   axis to the kept point and pays the step cost with the penalty shifted
//!   once more; it is blocked by a refused cell or a cost over what the
//!   search paid between the two plus ten. Points are cell centres on
//!   their layer's floor; fewer than two corners give no points.
//!
//! The original keeps its window, nodes and stamp table in shared buffers;
//! Exarust keeps one set per thread and clears the window's interior for
//! each search, as the original does. A grid smaller than the window reads
//! as no floor outside it. Evidence: reverse engineering of `Exanima.exe`
//! 0.9.5 (the planner, its line test, the squared-distance helper and the
//! window reset), `docs/NAVIGATION.md` (Planning).

use std::cell::RefCell;

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

/// A path on the grid: world points at cell centres on their floors, and
/// whether the search reached the goal (its accept distance, or an end
/// within 100 cm).
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub points: Vec<Vec3>,
    pub reached: bool,
    /// What the search paid to the path's end. Zero when it did not start.
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

/// The accept distance that stops a search once it meets the goal's cell
/// on the goal's height (ten heuristic units are one cell).
pub const ACCEPT_GOAL_CELL: i32 = 10;

/// Searches `grid` from `from` to `to` for a walker of `size` class: the
/// search stops at the goal's cell ([`ACCEPT_GOAL_CELL`]) and pays the
/// penalty bytes unshifted.
pub fn find_path(grid: &NavGrid, from: Vec3, to: Vec3, size: u8) -> Path {
    find_path_with(grid, from, to, size, ACCEPT_GOAL_CELL, 0)
}

/// The pathfinder's disturbance sampler: the original's stub always
/// answers 0.0.
pub fn disturbance() -> f32 {
    0.0
}

/// The eight neighbours in the original's order: north-west, north,
/// north-east, west, east, south-west, south, south-east, with their step
/// costs (7 on a diagonal, 5 orthogonally).
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

/// The window is 256 cells square; its centre cell is 127.
const WINDOW: i32 = 256;
const WINDOW_CENTRE: i32 = 127;
/// Start and goal lie at least this many cells inside the grid's half size.
const EDGE: i64 = 10;
/// Start and goal lie within this many cells of each other on each axis.
const MOST_APART: i64 = 253;
/// The nodes the search may make, the start's included (node 0 is none).
const MOST_NODES: usize = 4095;
/// A shifted penalty over this refuses a cell.
const MOST_PENALTY: i32 = 600;
/// A failed search's end this near the goal (cm², 100 cm) is reached.
const NEAR_ENOUGH_SQ: f32 = 10_000.0;
/// A failed path stops before the cells rougher than this over size + 1.
const ROUGH: i64 = 64;
/// A string-pulled line may cost this much more than the search's way.
const SHORTCUT_SLACK: i32 = 10;
/// A stamp the search may not enter (the window's border).
const BLOCKED: u16 = u16::MAX;

/// One node of the window.
#[derive(Debug, Clone, Copy)]
struct Node {
    /// Window coordinates.
    at: (i32, i32),
    layer: Layer,
    /// The entered layer's clearance byte when the node was made.
    clearance: u8,
    cost: i32,
    total: i32,
    /// The next node of the open list (0 ends it) and the parent.
    next: u16,
    parent: u16,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            at: (0, 0),
            layer: Layer::Lower,
            clearance: 0,
            cost: 0,
            total: 0,
            next: 0,
            parent: 0,
        }
    }
}

/// The per-thread buffers of the search: the stamp table (a node per window
/// place and layer, the border blocked), the nodes and the corner table.
#[derive(Default)]
struct Scratch {
    stamps: Vec<u16>,
    nodes: Vec<Node>,
    corners: Vec<u16>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

fn stamp_index(at: (i32, i32), layer: Layer) -> usize {
    ((at.1 * WINDOW + at.0) as usize) * 2 + layer as usize
}

/// Blocks the window's border once and clears its interior, rows and
/// columns 1 to 254, as the original's reset and search do.
fn reset_stamps(stamps: &mut Vec<u16>) {
    let side = WINDOW as usize;
    if stamps.len() != side * side * 2 {
        *stamps = vec![0; side * side * 2];
        for i in 0..WINDOW {
            for at in [(i, 0), (i, WINDOW - 1), (0, i), (WINDOW - 1, i)] {
                for layer in Layer::BOTH {
                    stamps[stamp_index(at, layer)] = BLOCKED;
                }
            }
        }
    }
    for row in 1..WINDOW - 1 {
        let start = stamp_index((1, row), Layer::Lower);
        stamps[start..start + (side - 2) * 2].fill(0);
    }
}

/// Metres to the original's centimetres, in single precision.
fn cm(metres: f32) -> f32 {
    super::centimetres_of(f64::from(metres)) as f32
}

/// A world coordinate (m) to a planner cell: `cm · (1/30) + ½`, rounded
/// ties to even.
fn planner_cell(metres: f32) -> i64 {
    let inverse = 1.0f32 / super::CELL_CM as f32;
    (cm(metres) * inverse + 0.5).round_ties_even() as i64
}

/// A height (m) in voxels over the grid's base: `(y − base) · 0.1`
/// (centimetres, the difference in single, the product in double),
/// rounded ties to even.
fn planner_height(y: f32, base: f32) -> i32 {
    (f64::from(cm(y) - cm(base)) * 0.1).round_ties_even() as i32
}

/// The cell at grid coordinates, or nothing outside the grid (the
/// original reads past a grid smaller than its window).
fn cells(grid: &NavGrid, x: i64, z: i64) -> Option<(NavCell, NavCell)> {
    let (width, depth) = grid.size();
    if x < 0 || z < 0 || x >= width as i64 || z >= depth as i64 {
        return None;
    }
    let (x, z) = (x as usize, z as usize);
    Some((grid.cell(x, z, Layer::Lower), grid.cell(x, z, Layer::Upper)))
}

/// The layer and floor a step from floor `own` enters (see the module's
/// Steps), before the rise and clearance tests.
fn step_onto(lower: NavCell, upper: NavCell, own: i32) -> Option<(Layer, i32)> {
    if lower.floor == 0 {
        return None;
    }
    let lower_floor = i32::from(lower.floor);
    // A lower floor out of reach, with room under its slab, leaves the
    // choice to the upper floor.
    let mut open = false;
    if own + 4 < lower_floor {
        if i32::from(lower.underside) < own + 10 {
            return None;
        }
        open = true;
    }
    let keep_lower = (!open).then_some((Layer::Lower, lower_floor));
    if upper.floor == 0 {
        return keep_lower;
    }
    let upper_floor = i32::from(upper.floor);
    if own + 4 < upper_floor {
        let under = i32::from(upper.underside);
        if under < own + 10 && (under != 0 || upper_floor < lower_floor) {
            return None;
        }
        return keep_lower;
    }
    if upper_floor <= lower_floor {
        // The line walk does not refuse here; a built grid's upper floor
        // always lies above its lower one, so the two never differ.
        return keep_lower;
    }
    Some((Layer::Upper, upper_floor))
}

/// `round(10·√(squares))`, the squares summed exactly.
fn heuristic(dx: i32, dz: i32, dh: i32) -> i32 {
    let squares = i64::from(dh.wrapping_mul(dh))
        + i64::from(dx.wrapping_mul(dx))
        + i64::from(dz.wrapping_mul(dz));
    ((squares as f64).sqrt() * 10.0).round_ties_even() as i64 as i32
}

/// The budget line walk from the far corner (`far`, on `layer`) to the
/// last point kept (`anchor`), grid coordinates: true when it is blocked.
fn line_blocked(
    grid: &NavGrid,
    far: (i64, i64),
    layer: Layer,
    anchor: (i64, i64),
    budget: i32,
    shift: u32,
) -> bool {
    let ends = [[far.0, far.1], [anchor.0, anchor.1]];
    let way = [(anchor.0 - far.0) as f32, (anchor.1 - far.1) as f32];
    let major = usize::from(way[0].abs() <= way[1].abs());
    let minor = 1 - major;
    let step = if way[major].is_sign_negative() { -1 } else { 1 };
    let count = (ends[1][major] - ends[0][major]).abs();
    if count == 0 {
        return false;
    }
    let slope = way[minor] / way[major];
    let intercept = ends[0][minor] as f32 - ends[0][major] as f32 * slope;
    let Some((lower, upper)) = cells(grid, far.0, far.1) else {
        return true;
    };
    let own = i32::from(if layer == Layer::Lower { lower } else { upper }.floor);
    let limit = budget.wrapping_add(SHORTCUT_SLACK);
    let mut at = ends[0];
    let mut cost = 0i32;
    // The cells after the start up to and including the kept point's own
    // cell: the original's loop runs once per cell of the major span.
    for _ in 0..count {
        let before = at[minor];
        at[major] += step;
        at[minor] = i64::from((at[major] as f32 * slope + intercept).round_ties_even() as i32);
        let way_cost = if before == at[minor] { 5 } else { 7 };
        let Some((lower, upper)) = cells(grid, at[0], at[1]) else {
            return true;
        };
        let Some((entered, floor)) = step_onto(lower, upper, own) else {
            return true;
        };
        let cell = if entered == Layer::Lower {
            lower
        } else {
            upper
        };
        let rise = floor - own;
        cost = cost
            .wrapping_add(rise * rise * 10 + way_cost + i32::from(cell.clearance) * 2)
            .wrapping_add(i32::from(cell.penalty).wrapping_shl(shift.wrapping_add(1)));
        if limit < cost {
            return true;
        }
    }
    false
}

/// Searches with an accept distance (the heuristic's units, ten a cell; 0
/// never accepts early) and the penalty byte's shift (masked to 31, as the
/// original's shift is). See the module for the rules.
pub fn find_path_with(
    grid: &NavGrid,
    from: Vec3,
    to: Vec3,
    size: u8,
    accept: i32,
    shift: u32,
) -> Path {
    SCRATCH.with(|scratch| {
        search(
            grid,
            from,
            to,
            size,
            accept,
            shift,
            &mut scratch.borrow_mut(),
        )
    })
}

fn search(
    grid: &NavGrid,
    from: Vec3,
    to: Vec3,
    size: u8,
    accept: i32,
    shift: u32,
    scratch: &mut Scratch,
) -> Path {
    let (width, depth) = grid.size();
    let goal = (planner_cell(to.x), planner_cell(to.z));
    let start = (planner_cell(from.x), planner_cell(from.z));
    let inside = |c: (i64, i64)| {
        c.0.abs() <= (width as i64 >> 1) - EDGE && c.1.abs() <= (depth as i64 >> 1) - EDGE
    };
    if !inside(goal)
        || !inside(start)
        || (goal.0 - start.0).abs() > MOST_APART
        || (goal.1 - start.1).abs() > MOST_APART
    {
        return Path::none();
    }
    let centre = (
        start.0 + (goal.0 - start.0) / 2,
        start.1 + (goal.1 - start.1) / 2,
    );
    let corner = (
        ((width as i64 - i64::from(WINDOW)) >> 1) + centre.0,
        ((depth as i64 - i64::from(WINDOW)) >> 1) + centre.1,
    );
    let place = |c: (i64, i64)| {
        (
            (c.0 - centre.0) as i32 + WINDOW_CENTRE,
            (c.1 - centre.1) as i32 + WINDOW_CENTRE,
        )
    };
    let (start_at, goal_at) = (place(start), place(goal));
    let grid_of = |at: (i32, i32)| (corner.0 + i64::from(at.0), corner.1 + i64::from(at.1));
    let (sx, sz) = grid_of(start_at);
    let Some((_, start_upper)) = cells(grid, sx, sz) else {
        return Path::none();
    };
    let goal_height = planner_height(to.y, grid.base());
    let start_height = i64::from(planner_height(from.y, grid.base()));
    let start_layer = if i64::from(start_upper.underside) - 3 < start_height
        && start_height < i64::from(start_upper.floor) + 15
    {
        Layer::Upper
    } else {
        Layer::Lower
    };
    let floor_of = |at: (i32, i32), layer: Layer| -> i32 {
        let (x, z) = grid_of(at);
        cells(grid, x, z).map_or(0, |(lower, upper)| {
            i32::from(if layer == Layer::Lower { lower } else { upper }.floor)
        })
    };

    reset_stamps(&mut scratch.stamps);
    let stamps = &mut scratch.stamps;
    let nodes = &mut scratch.nodes;
    nodes.clear();
    // The start's total is the original's, though it is never read: its
    // floor times 0.333 stands in for the height leg.
    let start_floor = f64::from(floor_of(start_at, start_layer)) * 0.333;
    let (dx, dz) = (start_at.0 - goal_at.0, start_at.1 - goal_at.1);
    let start_total =
        ((start_floor * start_floor + (i64::from(dx * dx) + i64::from(dz * dz)) as f64).sqrt()
            * 10.0)
            .round_ties_even() as i64 as i32;
    let first = Node {
        at: start_at,
        layer: start_layer,
        total: start_total,
        ..Node::default()
    };
    nodes.extend([Node::default(), first]);
    stamps[stamp_index(start_at, start_layer)] = 1;

    let limit = PASSABLE as i32 / (i32::from(size) + 1);
    let shift = shift & 31;
    let mut head: u16 = 0;
    let mut waiting: usize = 0;
    let mut nearest: u16 = 0;
    let mut nearest_h = i32::MAX;
    let mut current: u16 = 1;
    let mut reached = false;
    loop {
        let here = nodes[usize::from(current)];
        let own = floor_of(here.at, here.layer);
        for (dx, dz, way) in NEIGHBOURS {
            let at = (here.at.0 + dx, here.at.1 + dz);
            let (x, z) = grid_of(at);
            let Some((lower, upper)) = cells(grid, x, z) else {
                continue;
            };
            let Some((layer, floor)) = step_onto(lower, upper, own) else {
                continue;
            };
            let rise = floor - own;
            if !(-20..=4).contains(&rise) {
                continue;
            }
            let cell = if layer == Layer::Lower { lower } else { upper };
            if i32::from(cell.clearance) > limit {
                continue;
            }
            let penalty = i32::from(cell.penalty).wrapping_shl(shift);
            if penalty > MOST_PENALTY {
                continue;
            }
            let cost = (rise * rise * 10 + way + i32::from(cell.clearance) * 2)
                .wrapping_add(here.cost)
                .wrapping_add(penalty);
            let slot = stamp_index(at, layer);
            match stamps[slot] {
                0 => {
                    if nodes.len() > MOST_NODES {
                        // The 4096th node: nothing is left waiting.
                        waiting = 0;
                        break;
                    }
                    let id = nodes.len() as u16;
                    let h = heuristic(at.0 - goal_at.0, at.1 - goal_at.1, floor - goal_height);
                    if h < nearest_h {
                        nearest_h = h;
                        nearest = id;
                    }
                    nodes.push(Node {
                        at,
                        layer,
                        clearance: cell.clearance,
                        cost,
                        total: h.wrapping_add(cost),
                        next: head,
                        parent: current,
                    });
                    stamps[slot] = id;
                    head = id;
                    waiting += 1;
                }
                BLOCKED => {}
                old => {
                    let node = &mut nodes[usize::from(old)];
                    if i64::from(node.cost as u32) <= i64::from(cost) {
                        continue;
                    }
                    node.total = node.total.wrapping_sub(node.cost).wrapping_add(cost);
                    node.cost = cost;
                    node.parent = current;
                }
            }
        }
        if accept > nearest_h {
            reached = true;
            break;
        }
        if waiting == 0 {
            break;
        }
        // The waiting node of least total, compared unsigned; of equals the
        // newest (the list's front).
        let mut least = i64::from(i32::MAX);
        let (mut pick, mut pick_before) = (head, 0u16);
        let (mut scan, mut before) = (head, 0u16);
        for _ in 0..waiting {
            let node = nodes[usize::from(scan)];
            if i64::from(node.total as u32) < least {
                least = i64::from(node.total as u32);
                pick = scan;
                pick_before = before;
            }
            before = scan;
            scan = node.next;
        }
        if pick == head {
            head = nodes[usize::from(pick)].next;
        }
        nodes[usize::from(pick_before)].next = nodes[usize::from(pick)].next;
        waiting -= 1;
        current = pick;
    }
    if nearest == 0 {
        return Path::none();
    }

    let centre_cm = |node: &Node| -> [f32; 3] {
        let across = |c: i64, a: i32| ((c + i64::from(a)) as f32 - 127.5) * super::CELL_CM as f32;
        [
            across(centre.0, node.at.0),
            (floor_of(node.at, node.layer) * 10) as f32 + cm(grid.base()),
            across(centre.1, node.at.1),
        ]
    };
    let mut end = nearest;
    if !reached {
        let at = centre_cm(&nodes[usize::from(end)]);
        let goal_cm = [cm(to.x), cm(to.y), cm(to.z)];
        let d = [at[0] - goal_cm[0], at[1] - goal_cm[1], at[2] - goal_cm[2]];
        let squared = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        // Not farther than 100 cm; a NaN distance counts as near, as the
        // original's compare does.
        reached = squared.partial_cmp(&NEAR_ENOUGH_SQ) != Some(std::cmp::Ordering::Greater);
    }
    if !reached {
        let rough = (ROUGH / (i64::from(size) + 1)) as i32;
        let mut n = end;
        while n > 1 {
            let node = nodes[usize::from(n)];
            if i32::from(node.clearance) > rough {
                end = node.parent;
            }
            n = node.parent;
        }
    }

    // The corners, from the end (index 1) back to the start (the last).
    let corners = &mut scratch.corners;
    corners.clear();
    corners.extend([0, end]);
    let mut run = (0, 0);
    let mut id = end;
    for _ in 0..nodes.len() {
        if id == 1 {
            break;
        }
        let node = nodes[usize::from(id)];
        let back = node.parent;
        if back == id {
            break;
        }
        let next = nodes[usize::from(back)];
        let dir = (next.at.0 - node.at.0, next.at.1 - node.at.1);
        if dir != run || next.layer != node.layer {
            corners.push(back);
            run = dir;
        } else if let Some(last) = corners.last_mut() {
            *last = back;
        }
        id = back;
    }
    let count = corners.len() - 1;
    let cost = i64::from(nodes[usize::from(end)].cost);
    if count < 2 {
        return Path {
            points: Vec::new(),
            reached,
            cost,
        };
    }
    let point = |id: u16| {
        let node = nodes[usize::from(id)];
        let (x, z) = grid_of(node.at);
        let centre = grid.cell_centre(x as usize, z as usize);
        Vec3::new(
            centre.x,
            grid.floor_height(floor_of(node.at, node.layer) as u8),
            centre.y,
        )
    };
    let mut points = Vec::with_capacity(count);
    let mut kept = corners[count];
    points.push(point(kept));
    for i in (1..count.saturating_sub(1)).rev() {
        let far = nodes[usize::from(corners[i])];
        let anchor = nodes[usize::from(kept)];
        let paid = far.cost.wrapping_sub(anchor.cost);
        if line_blocked(
            grid,
            grid_of(far.at),
            far.layer,
            grid_of(anchor.at),
            paid,
            shift,
        ) {
            kept = corners[i + 1];
            points.push(point(kept));
        }
    }
    points.push(point(corners[1]));
    Path {
        points,
        reached,
        cost,
    }
}

#[cfg(test)]
#[path = "path_test.rs"]
mod tests;
