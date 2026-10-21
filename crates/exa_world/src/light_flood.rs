//! Lights stop at walls, as the original's do. Each light floods the 6 m
//! cell grid of a tile-grid level from its own cell, top down: what it
//! carries from cell to cell is an angular window, the arc of directions
//! still open from the light. Each side of a cell narrows the window to its
//! opening (wall stubs, corner cuts and doors make the opening smaller; solid
//! walls close it), and the flood keeps going while the window stays open
//! and the cell lies within the light's reach. A cell records the window it
//! was entered through, for at most 12 lights.
//!
//! A mesh is linked to a light only if its angular extent, seen from the
//! light, overlaps the window the light left in the mesh's cell or in a
//! neighbouring cell the mesh spills into. Rules: `docs/RENDERING.md`
//! (Lights stop at walls).

use std::f32::consts::{PI, TAU};

use bevy::math::{Mat3, Vec3};
use bevy::platform::collections::HashMap;

use crate::scene_nodes::{SPILL_HIGH, SPILL_LOW, cell_spill};

/// A window holding every direction (the light's own cell).
const WHOLE: f32 = 200.0;
/// Windows above this hold every direction.
const WHOLE_FROM: f32 = 100.0;
/// The most lights a cell records; later ones are dropped.
pub const MAX_CELL_LIGHTS: usize = 12;
/// The flood stops when its stack holds more entries than this.
const STACK_LIMIT: usize = 31;
/// Cell size, metres.
const CELL: f32 = 6.0;
/// Plane A's bit for a built cell.
const OPEN: u32 = 0x8_0000;

/// Outward normal and the axis along each side towards its a1 end, in cell
/// space (u along +X, v along +Z). Sides: 0 −Z, 1 −X, 2 +Z, 3 +X.
const OUT: [(f32, f32); 4] = [(0.0, -1.0), (-1.0, 0.0), (0.0, 1.0), (1.0, 0.0)];
const ALONG: [(f32, f32); 4] = [(1.0, 0.0), (0.0, -1.0), (-1.0, 0.0), (0.0, 1.0)];
/// The neighbour across each side.
const STEP: [(i64, i64); 4] = [(0, -1), (-1, 0), (0, 1), (1, 0)];
/// The half plane beyond each side, as a window.
const HALF: [(f32, f32); 4] = [
    (-PI / 2.0, PI / 2.0),
    (-PI, 0.0),
    (PI / 2.0, -PI / 2.0),
    (0.0, PI),
];

/// Adds a turn to a negative angle difference.
fn wrap(x: f32) -> f32 {
    if x < 0.0 { x + TAU } else { x }
}

/// Folds an angle difference into [−π, π].
fn fold(x: f32) -> f32 {
    if x > PI {
        x - TAU
    } else if x < -PI {
        x + TAU
    } else {
        x
    }
}

/// Whether arc (o0, o1) overlaps window (w0, w1), both swept towards
/// increasing angles.
fn overlaps(w0: f32, w1: f32, o0: f32, o1: f32) -> bool {
    let span = wrap(w1 - w0);
    let r0 = wrap(o0 - w0);
    let r1 = wrap(o1 - w0);
    r0 <= span || r1 <= r0
}

/// One end of a side's opening: position along the side, distance out from
/// the cell centre, and whether something narrows it there.
#[derive(Clone, Copy, Debug)]
struct End {
    t: f32,
    n: f32,
    narrowed: bool,
}

/// The planes of a tile grid that walls are read from.
#[derive(Clone, Debug)]
pub struct WallGrid {
    w: usize,
    h: usize,
    /// Plane A: per side a 4-bit edge code, per corner a 4-bit code.
    a: Vec<u32>,
    /// Plane C: special tiles in bits 16–20.
    c: Vec<u32>,
}

/// A light as the flood sees it.
#[derive(Clone, Copy, Debug)]
pub struct FloodLight {
    /// World position, metres.
    pub position: Vec3,
    /// Radius, metres.
    pub radius: f32,
    /// The vertical shaping's y, for lights shaped downwards.
    pub shaped: Option<f32>,
}

/// A mesh as the wall test sees it (world metres).
#[derive(Clone, Copy, Debug)]
pub struct NodeBounds {
    pub centre: Vec3,
    pub radius: f32,
    /// Half extents of its box along world X and Z.
    pub extent_x: f32,
    pub extent_z: f32,
}

impl WallGrid {
    /// From the grid's size and planes A and C; `None` when they disagree or
    /// the grid is too small to hold a cell.
    pub fn new(w: usize, h: usize, a: Vec<u32>, c: Vec<u32>) -> Option<Self> {
        (w >= 3 && h >= 3 && a.len() == w * h && c.len() == w * h).then_some(Self { w, h, a, c })
    }

    fn index(&self, i: i64, j: i64) -> Option<usize> {
        (i >= 0 && j >= 0 && (i as usize) < self.w && (j as usize) < self.h)
            .then(|| j as usize * self.w + i as usize)
    }

    /// The grid's size in cells (X, Z).
    pub fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    /// Whether a cell is built: plane A's open flag `0x80000`. A cell
    /// outside the grid is not.
    pub fn open(&self, x: i64, z: i64) -> bool {
        self.index(x, z).is_some_and(|i| self.a[i] & OPEN != 0)
    }

    /// World metres to cell space: cell (i, j) is centred on (i, j).
    fn to_cells(&self, x: f32, z: f32) -> (f32, f32) {
        (
            x / CELL + (self.w >> 1) as f32 - 0.5,
            z / CELL + (self.h >> 1) as f32 - 0.5,
        )
    }

    fn edge(&self, cell: usize, side: usize) -> u32 {
        (self.a[cell] >> (4 * side)) & 0xF
    }

    fn corner_cut(&self, cell: usize, corner: usize) -> bool {
        self.a[cell] & (0x40000 << (4 * corner)) != 0
    }

    /// The two ends of side `side`'s opening, or `None` for a solid side.
    fn ends(&self, cell: usize, side: usize) -> Option<[End; 2]> {
        let e = self.edge(cell, side);
        if e & 0x4 != 0 {
            return None;
        }
        if e & 0x8 != 0 {
            // Doors: a fixed opening, wider in cells with a special tile.
            let t = if self.c[cell] & 0x1F_0000 != 0 {
                0.35
            } else {
                0.2
            };
            let end = |t| End {
                t,
                n: 0.51,
                narrowed: true,
            };
            return Some([end(-t), end(t)]);
        }
        let a0 = if e & 0x1 != 0 {
            End {
                t: -0.2,
                n: 0.51,
                narrowed: true,
            }
        } else if self.corner_cut(cell, side) {
            End {
                t: -0.4,
                n: 0.51,
                narrowed: true,
            }
        } else {
            End {
                t: -0.5,
                n: 0.5,
                narrowed: false,
            }
        };
        let a1 = if e & 0x2 != 0 {
            End {
                t: 0.2,
                n: 0.51,
                narrowed: true,
            }
        } else if self.corner_cut(cell, (side + 3) & 3) {
            End {
                t: 0.4,
                n: 0.51,
                narrowed: true,
            }
        } else {
            End {
                t: 0.5,
                n: 0.5,
                narrowed: false,
            }
        };
        Some([a0, a1])
    }
}

/// The direction of point `p` from the light at `l`, top view, cell space:
/// 0 towards −Z, π/2 towards +X.
fn angle(l: (f32, f32), p: (f32, f32)) -> f32 {
    (p.0 - l.0).atan2(l.1 - p.1)
}

/// The angles of a side's opening ends, seen from the light.
fn opening(cell: (i64, i64), side: usize, ends: [End; 2], l: (f32, f32)) -> [f32; 2] {
    ends.map(|e| {
        let p = (
            cell.0 as f32 + e.n * OUT[side].0 + e.t * ALONG[side].0,
            cell.1 as f32 + e.n * OUT[side].1 + e.t * ALONG[side].1,
        );
        angle(l, p)
    })
}

/// The lights every cell records: (light key, window).
#[derive(Clone, Debug, Default)]
pub struct CellLights {
    cells: Vec<Vec<(u32, f32, f32)>>,
    /// The cells each light entered.
    touched: HashMap<u32, Vec<usize>>,
}

impl CellLights {
    pub fn new(grid: &WallGrid) -> Self {
        Self {
            cells: vec![Vec::new(); grid.w * grid.h],
            touched: HashMap::default(),
        }
    }

    /// Takes light `key` out of every cell. Each cell drops its first entry
    /// for the light by swapping the last entry into its place (the
    /// original scans every cell the same way), so the order of the other
    /// lights in a cell can change.
    pub fn remove(&mut self, key: u32) {
        for cell in self.touched.remove(&key).unwrap_or_default() {
            let list = &mut self.cells[cell];
            if let Some(at) = list.iter().position(|e| e.0 == key) {
                list.swap_remove(at);
            }
        }
    }

    /// Puts a directional light (one that is not flooded) in every cell,
    /// as the original does: a cell that does not list the light yet and
    /// holds fewer than [`MAX_CELL_LIGHTS`] gets it with an empty window; a
    /// cell that lists it keeps its window. The original walks `width²`
    /// cells, so a grid with more rows than columns keeps its last rows
    /// empty; one with fewer rows is walked to its end here instead of past
    /// it.
    pub fn fill_directional(&mut self, grid: &WallGrid, key: u32) {
        let count = (grid.w * grid.w).min(self.cells.len());
        for cell in 0..count {
            let list = &mut self.cells[cell];
            if list.iter().any(|e| e.0 == key) || list.len() >= MAX_CELL_LIGHTS {
                continue;
            }
            list.push((key, 0.0, 0.0));
            self.touched.entry(key).or_default().push(cell);
        }
    }

    fn insert(&mut self, cell: usize, key: u32, a0: f32, a1: f32) {
        let list = &mut self.cells[cell];
        if let Some(e) = list.iter_mut().find(|e| e.0 == key) {
            if e.1 > WHOLE_FROM {
                return;
            }
            if a0 > WHOLE_FROM {
                (e.1, e.2) = (a0, a1);
                return;
            }
            // Each end widens on its own: the union of overlapping arcs.
            if fold(a0 - e.1) < 0.0 {
                e.1 = a0;
            }
            if fold(a1 - e.2) > 0.0 {
                e.2 = a1;
            }
        } else if list.len() < MAX_CELL_LIGHTS {
            list.push((key, a0, a1));
            self.touched.entry(key).or_default().push(cell);
        }
    }

    fn window(&self, cell: usize, key: u32) -> Option<(f32, f32)> {
        self.cells[cell]
            .iter()
            .find(|e| e.0 == key)
            .map(|e| (e.1, e.2))
    }

    /// The lights a cell records.
    pub fn lights(&self, cell: usize) -> impl Iterator<Item = u32> + '_ {
        self.cells[cell].iter().map(|e| e.0)
    }

    /// Floods light `key` through `grid`, replacing what it left before.
    pub fn flood(&mut self, grid: &WallGrid, key: u32, light: &FloodLight) {
        self.remove(key);
        let l = grid.to_cells(light.position.x, light.position.z);
        let reach = light.radius / CELL;
        let start = (
            ((l.0 + 0.5).trunc() as i64).clamp(1, grid.w as i64 - 2),
            ((l.1 + 0.5).trunc() as i64).clamp(1, grid.h as i64 - 2),
        );
        let Some(start_cell) = grid.index(start.0, start.1) else {
            return;
        };
        self.insert(start_cell, key, WHOLE, WHOLE);
        let mut stack: Vec<((i64, i64), f32, f32)> = Vec::new();
        for side in [3, 2, 1, 0] {
            let Some(ends) = grid.ends(start_cell, side) else {
                continue;
            };
            let o = opening(start, side, ends, l);
            let (mut w0, mut w1) = HALF[side];
            if ends[0].narrowed {
                w0 = o[0];
            }
            if ends[1].narrowed {
                w1 = o[1];
            }
            let next = (start.0 + STEP[side].0, start.1 + STEP[side].1);
            stack.push((next, w0, w1));
        }
        while stack.len() <= STACK_LIMIT {
            let Some((cell, w0, w1)) = stack.pop() else {
                break;
            };
            let Some(index) = grid.index(cell.0, cell.1) else {
                continue;
            };
            let d2 = (cell.0 as f32 - l.0).powi(2) + (cell.1 as f32 - l.1).powi(2);
            if d2 > (reach + 0.5).powi(2) {
                continue;
            }
            if let Some(y) = light.shaped {
                // Downward-shaped lights reach out only as far as their
                // shaping lets them at the light's height.
                let h = ((light.position.y * 100.0 + 1000.0) / 600.0).min(reach);
                let c = h / (h * h + d2).sqrt().max(1e-6);
                if c + (1.0 - c) * y < 1.19e-7 {
                    continue;
                }
            }
            self.insert(index, key, w0, w1);
            for side in [3, 2, 1, 0] {
                // Only sides facing away from the light.
                let inward = (-OUT[side].0, -OUT[side].1);
                let at = cell.0 as f32 * inward.0 + cell.1 as f32 * inward.1;
                if at - 0.5 >= l.0 * inward.0 + l.1 * inward.1 {
                    continue;
                }
                let Some(ends) = grid.ends(index, side) else {
                    continue;
                };
                let [mut o0, mut o1] = opening(cell, side, ends, l);
                let span = wrap(w1 - w0);
                let r0 = wrap(o0 - w0);
                let r1 = wrap(o1 - w0);
                if !(r0 <= span || r1 <= r0) {
                    continue;
                }
                if r1 < r0 {
                    o0 = w0;
                }
                if span < r1 {
                    o1 = w1;
                }
                let next = (cell.0 + STEP[side].0, cell.1 + STEP[side].1);
                stack.push((next, o0, o1));
            }
        }
    }

    /// Whether light `key` at `light` (metres) sees the mesh `node` through
    /// the windows it left in the mesh's cell or the cells it spills into.
    pub fn sees(&self, grid: &WallGrid, key: u32, light: Vec3, node: &NodeBounds) -> bool {
        let d = node.centre - light;
        let dist = d.length();
        if dist < node.radius {
            return true;
        }
        let centre = d.x.atan2(-d.z);
        let half = (node.radius / dist).atan();
        let mut n0 = centre - half;
        if n0 < -PI {
            n0 += TAU;
        }
        let mut n1 = centre + half;
        if n1 > PI {
            n1 -= TAU;
        }
        // The node's cell and which neighbours its box spills into, exactly
        // as the scene's cell update works them out.
        let spill = cell_spill(
            node.centre,
            Vec3::new(node.extent_x, 0.0, node.extent_z),
            Mat3::IDENTITY,
            (grid.w, grid.h),
        );
        let Some((ci, cj)) = spill.cell else {
            return false;
        };
        let xn = spill.mask & SPILL_LOW != 0;
        let xp = spill.mask & SPILL_HIGH != 0;
        let zn = spill.mask & (SPILL_LOW << 1) != 0;
        let zp = spill.mask & (SPILL_HIGH << 1) != 0;
        let Some(own) = grid.index(ci, cj) else {
            return false;
        };
        let walled = |side: usize| grid.edge(own, side) & 0x7 == 0x7;
        for dj in -1i64..=1 {
            for di in -1i64..=1 {
                let spills_x = match di {
                    -1 => xn,
                    1 => xp,
                    _ => true,
                };
                let spills_z = match dj {
                    -1 => zn,
                    1 => zp,
                    _ => true,
                };
                if !(spills_x && spills_z) {
                    continue;
                }
                let wall_x = match di {
                    -1 => walled(1),
                    1 => walled(3),
                    _ => false,
                };
                let wall_z = match dj {
                    -1 => walled(0),
                    1 => walled(2),
                    _ => false,
                };
                // A wall closes its side; a diagonal only both walls.
                let blocked = if di != 0 && dj != 0 {
                    wall_x && wall_z
                } else {
                    wall_x || wall_z
                };
                if blocked {
                    continue;
                }
                let Some(cell) = grid.index(ci + di, cj + dj) else {
                    continue;
                };
                let Some((a0, a1)) = self.window(cell, key) else {
                    continue;
                };
                if a0 > WHOLE_FROM || overlaps(a0, a1, n0, n1) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLID: u32 = 0x7;
    const DOOR: u32 = 0xB;

    /// A 10 × 6 grid: room 1 (columns 1–3) and room 2 (columns 5–7) of
    /// rows 1–3, joined by a corridor cell (4, 2) that opens onto room 2 and
    /// meets room 1 through a door or a wall. Every other cell is solid.
    fn rooms(door: bool) -> WallGrid {
        let (w, h) = (10usize, 6usize);
        let room = |i: i64, j: i64| -> Option<u8> {
            match (i, j) {
                (1..=3, 1..=3) => Some(1),
                (5..=7, 1..=3) => Some(2),
                (4, 2) => Some(3),
                _ => None,
            }
        };
        let mut a = vec![0xFFFFu32; w * h];
        for j in 0..h as i64 {
            for i in 0..w as i64 {
                let Some(r) = room(i, j) else { continue };
                let mut code = 0u32;
                for (side, (di, dj)) in STEP.iter().enumerate() {
                    let e = match (r, room(i + di, j + dj)) {
                        (x, Some(y)) if x == y => 0,
                        (3, Some(2)) | (2, Some(3)) => 0,
                        (3, Some(1)) | (1, Some(3)) if door => DOOR,
                        _ => SOLID,
                    };
                    code |= e << (4 * side);
                }
                a[j as usize * w + i as usize] = code;
            }
        }
        WallGrid::new(w, h, a, vec![0; w * h]).unwrap()
    }

    /// The world centre (metres) of cell (i, j).
    fn centre(g: &WallGrid, i: usize, j: usize) -> Vec3 {
        Vec3::new(
            ((i as f32 - (g.w >> 1) as f32) + 0.5) * CELL,
            1.5,
            ((j as f32 - (g.h >> 1) as f32) + 0.5) * CELL,
        )
    }

    fn torch(at: Vec3) -> FloodLight {
        FloodLight {
            position: at,
            radius: 24.0,
            shaped: None,
        }
    }

    fn cell(g: &WallGrid, i: usize, j: usize) -> usize {
        j * g.w + i
    }

    #[test]
    fn walls_stop_the_flood() {
        let g = rooms(false);
        let mut cells = CellLights::new(&g);
        cells.flood(&g, 1, &torch(centre(&g, 2, 2)));
        for j in 1..=3 {
            for i in 1..=3 {
                assert!(
                    cells.window(cell(&g, i, j), 1).is_some(),
                    "room cell {i},{j}"
                );
            }
        }
        assert!(cells.window(cell(&g, 4, 2), 1).is_none());
        assert!(cells.window(cell(&g, 6, 2), 1).is_none());
        // Its own cell sees everything.
        assert!(cells.window(cell(&g, 2, 2), 1).unwrap().0 > WHOLE_FROM);
    }

    #[test]
    fn doors_let_light_through() {
        let g = rooms(true);
        let mut cells = CellLights::new(&g);
        cells.flood(&g, 1, &torch(centre(&g, 2, 2)));
        assert!(cells.window(cell(&g, 4, 2), 1).is_some());
        assert!(cells.window(cell(&g, 5, 2), 1).is_some());
        // Past the door only a narrow beam carries on.
        let (a0, a1) = cells.window(cell(&g, 5, 2), 1).unwrap();
        assert!(wrap(a1 - a0) < 0.5, "{a0} {a1}");
    }

    #[test]
    fn meshes_behind_walls_are_not_seen() {
        let g = rooms(false);
        let mut cells = CellLights::new(&g);
        let light = centre(&g, 2, 2);
        cells.flood(&g, 1, &torch(light));
        let node = |i, j| NodeBounds {
            centre: centre(&g, i, j),
            radius: 0.5,
            extent_x: 0.3,
            extent_z: 0.3,
        };
        assert!(cells.sees(&g, 1, light, &node(1, 1)));
        assert!(!cells.sees(&g, 1, light, &node(6, 2)));
        // A mesh around the light always is.
        let mut big = node(6, 2);
        big.centre = light + Vec3::X * 0.2;
        assert!(cells.sees(&g, 1, light, &big));
    }

    #[test]
    fn cells_hold_twelve_lights() {
        let g = rooms(false);
        let mut cells = CellLights::new(&g);
        for key in 0..14 {
            cells.flood(&g, key, &torch(centre(&g, 2, 2)));
        }
        assert_eq!(cells.lights(cell(&g, 1, 1)).count(), MAX_CELL_LIGHTS);
        assert!(cells.window(cell(&g, 1, 1), 13).is_none());
        // Moving a light out frees its place.
        cells.remove(0);
        assert_eq!(cells.lights(cell(&g, 1, 1)).count(), MAX_CELL_LIGHTS - 1);
    }

    #[test]
    fn removing_a_light_swaps_the_last_one_into_its_place() {
        let g = rooms(false);
        let mut cells = CellLights::new(&g);
        for key in 1..=3 {
            cells.insert(0, key, 0.0, 1.0);
        }
        cells.remove(1);
        assert_eq!(cells.lights(0).collect::<Vec<_>>(), [3, 2]);
    }

    #[test]
    fn a_directional_light_sits_in_every_cell() {
        let g = rooms(false);
        let mut cells = CellLights::new(&g);
        // A cell that lists the light already keeps its window.
        cells.insert(0, 7, 1.0, 2.0);
        cells.fill_directional(&g, 7);
        assert_eq!(cells.window(0, 7), Some((1.0, 2.0)));
        for cell in 1..g.w * g.h {
            assert_eq!(cells.window(cell, 7), Some((0.0, 0.0)), "cell {cell}");
        }
        // The twelve-light cap holds for the fill too.
        for key in 0..13 {
            cells.fill_directional(&g, key);
        }
        assert_eq!(cells.lights(1).count(), MAX_CELL_LIGHTS);
        // Removing a filled light takes it out of every cell.
        cells.remove(7);
        assert_eq!(cells.lights(1).count(), MAX_CELL_LIGHTS - 1);
        assert!(cells.window(1, 7).is_none());
        // A grid taller than wide is filled over width² cells only.
        let tall = WallGrid::new(3, 5, vec![0; 15], vec![0; 15]).unwrap();
        let mut cells = CellLights::new(&tall);
        cells.fill_directional(&tall, 1);
        assert_eq!(cells.window(8, 1), Some((0.0, 0.0)));
        assert!(cells.window(9, 1).is_none());
    }

    #[test]
    fn windows_widen_by_union() {
        let g = rooms(false);
        let mut cells = CellLights::new(&g);
        cells.insert(0, 1, 0.0, 0.5);
        cells.insert(0, 1, 0.3, 1.0);
        assert_eq!(cells.window(0, 1), Some((0.0, 1.0)));
        cells.insert(0, 1, -0.4, 0.2);
        assert_eq!(cells.window(0, 1), Some((-0.4, 1.0)));
    }

    #[test]
    fn arcs_overlap_across_the_back() {
        assert!(overlaps(3.0, -3.0, 3.1, 3.2));
        assert!(overlaps(3.0, -3.0, -3.1, -2.0));
        assert!(!overlaps(-1.0, 1.0, 2.0, 2.5));
        assert!(overlaps(-1.0, 1.0, 0.9, 2.0));
    }
}
