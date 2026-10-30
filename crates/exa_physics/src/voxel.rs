//! The mesh voxeliser and its voxel map: the grid of cells a mesh is sampled
//! into, the points each generator extracts from it, and the ray query that
//! finds the highest surface over a column.
//!
//! The voxel map is the container of the rigid-body voxeliser: a bordered
//! box of cells (`VoxelMap`), one word and two fill bytes per cell, and the
//! spheres extracted from them. The shape builders fill one per mesh; the
//! grid fill and the sphere extraction are theirs, not the map's.
//!
//! Two generators sample a mesh into a fine grid and turn the occupied
//! cells into points on a coarser lattice:
//!
//! * [`floor_points`] is the floor heightfield: rays down each fine column,
//!   the highest upward face wins, its 10 cm cell is marked, and every
//!   20 cm cell holding four marks gives a point. It keeps the original's
//!   centimetres and its space, a grid cell's merged floor: its edges start
//!   from the cell's quarters (±150/±300 cm), snap to 20 cm, and its point
//!   step is a hard-coded 20 cm.
//! * [`surface_points`] is the general surface generator: cells around the
//!   mesh are classified by a sphere query, the near ones are cleaned up,
//!   and a greedy cover (or one point per cell) turns them into points at a
//!   step of a fifth of the generator's parameter (4 cm for the original's
//!   20).
//!
//! Rules: `docs/RENDERING.md` (Voxel GI).

use std::sync::Arc;

use bevy::math::Vec3;

/// The resolution the original's voxel map is made with.
pub const MAP_RESOLUTION: f32 = 0.02;
/// The scale the original's voxel map is made with.
pub const MAP_SCALE: f32 = 1.0;

/// A collision sphere the voxeliser extracts from a mesh, in the map's
/// space: its centre and radius (the original's 16-byte record).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub centre: Vec3,
    pub radius: f32,
}

/// The voxel map of the rigid-body voxeliser: a box over a mesh cut into
/// cells, one word per cell, two fill bytes per cell and the spheres
/// extracted from them.
///
/// The grid carries one empty border cell on each side of the box's
/// `counts`, and its cells run Z fastest, then Y, then X, as the shape
/// builder's grid fill lays them out. Cell `i` (border included) has its
/// centre at `i · step + (centre − half) − half / count` on each axis, with
/// `step = 2 · half / count`: the border cells sit just outside the box.
///
/// The original makes one per shape-builder call. Its array release frees
/// the cell words and the fill bytes; its destructor frees those and then
/// the sphere list. Here the map owns all three and [`VoxelMap::release`]
/// is the array release.
#[derive(Debug, Clone)]
pub struct VoxelMap {
    /// The map's resolution (0.02 at construction; the shape builder
    /// chooses the working value from the mesh's size).
    pub resolution: f32,
    /// The map's unit scale (1 at construction).
    pub scale: f32,
    /// What the map was made for: the caller's word, kept as given.
    pub tag: u32,
    /// Cells per axis inside the box, without the border.
    pub counts: [usize; 3],
    /// Centre of the map's box, in the map's space.
    pub centre: Vec3,
    /// Half extents of the map's box.
    pub half_extents: Vec3,
    cells: Vec<i32>,
    fill: Vec<[u8; 2]>,
    spheres: Vec<Sphere>,
}

impl Default for VoxelMap {
    fn default() -> Self {
        Self::new(0)
    }
}

impl VoxelMap {
    /// An empty map made for `tag`, at the original's resolution and scale.
    pub fn new(tag: u32) -> Self {
        Self {
            resolution: MAP_RESOLUTION,
            scale: MAP_SCALE,
            tag,
            counts: [0; 3],
            centre: Vec3::ZERO,
            half_extents: Vec3::ZERO,
            cells: Vec::new(),
            fill: Vec::new(),
            spheres: Vec::new(),
        }
    }

    /// Sets the map's box and its cell counts inside it, and makes the
    /// bordered grid of cell words, all zero. Returns false (and leaves the
    /// map empty) when the grid does not fit in memory.
    pub fn resize(&mut self, centre: Vec3, half_extents: Vec3, counts: [usize; 3]) -> bool {
        self.centre = centre;
        self.half_extents = half_extents;
        self.counts = counts;
        self.cells = Vec::new();
        self.fill = Vec::new();
        let Some(len) = counts
            .iter()
            .try_fold(1usize, |n, &d| {
                d.checked_add(2).and_then(|d| n.checked_mul(d))
            })
            .filter(|&n| n <= isize::MAX as usize / 4)
        else {
            self.counts = [0; 3];
            return false;
        };
        if self.cells.try_reserve_exact(len).is_err() {
            self.counts = [0; 3];
            return false;
        }
        self.cells.resize(len, 0);
        true
    }

    /// Cells per axis with the border: `counts + 2`.
    pub fn dims(&self) -> [usize; 3] {
        self.counts.map(|n| n + 2)
    }

    /// One cell's size per axis: `2 · half / count`.
    pub fn step(&self) -> Vec3 {
        let n = self.counts.map(|n| n as f32);
        Vec3::new(
            self.half_extents.x * 2.0 / n[0],
            self.half_extents.y * 2.0 / n[1],
            self.half_extents.z * 2.0 / n[2],
        )
    }

    /// The linear index of a cell (border included), Z fastest, then Y,
    /// then X; `None` outside the grid.
    pub fn index(&self, cell: [usize; 3]) -> Option<usize> {
        let [nx, ny, nz] = self.dims();
        (cell[0] < nx && cell[1] < ny && cell[2] < nz && !self.cells.is_empty())
            .then(|| cell[2] + nz * (cell[1] + ny * cell[0]))
    }

    /// The centre of a cell (border included), or `None` outside the grid.
    pub fn cell_centre(&self, cell: [usize; 3]) -> Option<Vec3> {
        self.index(cell)?;
        let step = self.step();
        let low = |a: usize| {
            let h = self.half_extents[a];
            (self.centre[a] - h) - h / self.counts[a] as f32
        };
        Some(Vec3::new(
            cell[0] as f32 * step.x + low(0),
            cell[1] as f32 * step.y + low(1),
            cell[2] as f32 * step.z + low(2),
        ))
    }

    /// A cell's word, or `None` outside the grid.
    pub fn cell(&self, cell: [usize; 3]) -> Option<i32> {
        self.index(cell).map(|i| self.cells[i])
    }

    /// Sets a cell's word; false outside the grid.
    pub fn set_cell(&mut self, cell: [usize; 3], value: i32) -> bool {
        match self.index(cell) {
            Some(i) => {
                self.cells[i] = value;
                true
            }
            None => false,
        }
    }

    /// Every cell's word, Z fastest.
    pub fn cells(&self) -> &[i32] {
        &self.cells
    }

    /// Every cell's word, writable.
    pub fn cells_mut(&mut self) -> &mut [i32] {
        &mut self.cells
    }

    /// The two fill bytes of every cell. The sphere extraction reads them
    /// (a first byte above 7 marks a solid cell); which stage of the shape
    /// builder writes them is not established.
    pub fn fill(&self) -> &[[u8; 2]] {
        &self.fill
    }

    /// Replaces the fill bytes.
    pub fn set_fill(&mut self, fill: Vec<[u8; 2]>) {
        self.fill = fill;
    }

    /// The spheres extracted from the cells.
    pub fn spheres(&self) -> &[Sphere] {
        &self.spheres
    }

    /// Replaces the extracted spheres.
    pub fn set_spheres(&mut self, spheres: Vec<Sphere>) {
        self.spheres = spheres;
    }

    /// Frees the cell words and the fill bytes, as the original's array
    /// release does; the spheres stay until the map goes.
    pub fn release(&mut self) {
        self.cells = Vec::new();
        self.fill = Vec::new();
    }
}

/// A mesh's bounds in its own space: the centre and half extents the
/// generators work from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub centre: Vec3,
    pub half_extents: Vec3,
}

/// The smallest vertical distance a face may have under a ray (the
/// original's epsilon, a hair under `f32::EPSILON`).
pub const SURFACE_EPSILON: f32 = 1.1920899822825959e-07;

/// One upward face set up for the down-ray query: its plane scaled so that
/// `y`'s coefficient is one, the two axes the barycentric test uses, the
/// first vertex's coordinates on them, and the inverse of the 2×2 basis the
/// test solves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceTriangle {
    plane: [f32; 4],
    axes: [usize; 2],
    first: [f32; 2],
    weights: [f32; 4],
    /// The face's x and z range (`[x_min, x_max, z_min, z_max]`) when it
    /// is tested on x and z; `None` for a steep face tested on y.
    footprint: Option<[f32; 4]>,
}

/// A face's plane as the mesh stores it: the normal of `(b − a) × (c − a)`
/// and the plane's constant `a · normal`, summed in the original's order.
///
/// The original scales the cross product by the hardware's approximate
/// reciprocal square root; this takes the exact one. The plane is scaled by
/// one over its normal's y before use, so the two differ only in rounding.
fn stored_plane([a, b, c]: [Vec3; 3]) -> [f32; 4] {
    let u = b - a;
    let v = c - a;
    let cross = Vec3::new(
        v.z * u.y - u.z * v.y,
        v.x * u.z - u.x * v.z,
        v.y * u.x - u.y * v.x,
    );
    let length_squared = cross.x * cross.x + cross.y * cross.y + cross.z * cross.z;
    let r = if length_squared == 0.0 {
        0.0
    } else {
        1.0 / length_squared.sqrt()
    };
    let constant = a.x * cross.x * r + a.y * cross.y * r + a.z * cross.z * r;
    [cross.x * r, cross.y * r, cross.z * r, constant]
}

impl SurfaceTriangle {
    /// Sets a triangle up for [`surface_hit`], or returns `None` when it
    /// does not face up (normal y below the epsilon, as the original keeps
    /// only upward faces; a degenerate face has a zero normal).
    pub fn new(triangle: [Vec3; 3]) -> Option<Self> {
        let [a, b, c] = triangle;
        let stored = stored_plane(triangle);
        if stored[1] < SURFACE_EPSILON {
            return None;
        }
        // The dominant axis, first in the comparison order the original uses.
        let n = [stored[0].abs(), stored[1].abs(), stored[2].abs()];
        let dominant = if n[0] <= n[1] {
            if n[1] <= n[2] { 2 } else { 1 }
        } else if n[0] <= n[2] {
            2
        } else {
            0
        };
        let axes = [(dominant + 1) % 3, (dominant + 2) % 3];
        let (axis1, axis2) = (axes[0], axes[1]);
        // The stored plane's four coefficients scaled by one over its y, so
        // y's coefficient is `n.y · (1 / n.y)`, not necessarily exactly one.
        let scale = 1.0 / stored[1];
        let plane = stored.map(|v| v * scale);
        // The edges from the first vertex, as the original lays them out.
        let edge_c = (c - a).to_array();
        let edge_b = (b - a).to_array();
        let f = 1.0 / (edge_c[axis1] * edge_b[axis2] - edge_c[axis2] * edge_b[axis1]);
        let weights = [
            -edge_c[axis2] * f,
            edge_c[axis1] * f,
            edge_b[axis2] * f,
            -edge_b[axis1] * f,
        ];
        let first = a.to_array();
        let footprint = (dominant == 1).then(|| {
            [
                a.x.min(b.x).min(c.x),
                a.x.max(b.x).max(c.x),
                a.z.min(b.z).min(c.z),
                a.z.max(b.z).max(c.z),
            ]
        });
        Some(Self {
            plane,
            axes,
            first: [first[axis1], first[axis2]],
            weights,
            footprint,
        })
    }
}

/// The distance from `origin` down to a face's plane, summed as the
/// original does: `(a·x + b·y) + c·z − d`.
fn plane_distance(plane: &[f32; 4], origin: [f32; 3]) -> f32 {
    (plane[0] * origin[0] + plane[1] * origin[1]) + plane[2] * origin[2] - plane[3]
}

/// The first face hit by a ray down `direction`, with the distance from
/// `origin` to its plane: the highest face wins, the first of equal
/// distances stays, and the barycentric test takes the edges as inside.
pub fn surface_hit(
    faces: &[SurfaceTriangle],
    origin: Vec3,
    direction: Vec3,
    max: f32,
) -> Option<f32> {
    hit_among(faces.iter(), origin, direction, max)
}

/// [`surface_hit`] over the faces `faces` yields, in that order.
fn hit_among<'a>(
    faces: impl Iterator<Item = &'a SurfaceTriangle>,
    origin: Vec3,
    direction: Vec3,
    max: f32,
) -> Option<f32> {
    let origin = origin.to_array();
    let direction = direction.to_array();
    let mut best = max;
    let mut hit = false;
    for face in faces {
        let distance = plane_distance(&face.plane, origin);
        if distance.is_nan() || distance <= SURFACE_EPSILON || distance >= best {
            continue;
        }
        let [axis1, axis2] = face.axes;
        let along_1 = (origin[axis1] + direction[axis1] * distance) - face.first[0];
        let along_2 = (origin[axis2] + direction[axis2] * distance) - face.first[1];
        let w1 = face.weights[1] * along_2 + face.weights[0] * along_1;
        if w1.is_nan() || w1 < 0.0 {
            continue;
        }
        let w2 = face.weights[3] * along_2 + face.weights[2] * along_1;
        if w2.is_nan() || w2 < 0.0 || w1 + w2 > 1.0 {
            continue;
        }
        best = distance;
        hit = true;
    }
    hit.then_some(best)
}

/// The bounds edges the floor heightfield snaps to, in centimetres: the low
/// edge takes the first of the first four that is more than 75 cm above the
/// mesh's low corner, the high edge the first of the last four (downwards)
/// that is more than 75 cm below the mesh's high corner.
const FLOOR_EDGES: [f32; 5] = [-300.0, -150.0, 0.0, 150.0, 300.0];
/// The finest cell of the floor heightfield: half of the generator's
/// parameter, 10 cm for the original's 20.
const FLOOR_FINE_SHARE: f32 = 0.5;
/// The floor points' step, hard-coded in the original's downsample.
pub const FLOOR_POINT_STEP: f32 = 20.0;
/// Marks a 2×2×2 block of fine cells needs (four columns of 255) before it
/// gives a point.
const FLOOR_MARK_SUM: u32 = 1019;

/// Rounds to the nearest whole number, ties to even, as the original's
/// `Round` does, and converts to an integer the way a cast does (no panic).
fn round_ties_even(v: f32) -> i64 {
    v.round_ties_even() as i64
}

/// A floor edge snapped to the 20 cm lattice: `round(v / 20 ∓ 0.1) · 20`,
/// the fraction computed in the original's mixed single/double constants
/// and the product taken in whole numbers.
fn snap_edge(v: f32, offset: f64) -> f32 {
    let steps = ((v * 0.05f32) as f64 - offset).round_ties_even() as i64;
    steps.wrapping_mul(20) as f32
}

/// The floor heightfield of a mesh: a point at the centre of every 20 cm
/// cell whose four 10 cm columns all meet a surface in the same 20 cm band.
///
/// `bounds` is the mesh's own bounds and `triangles` its faces, both in
/// centimetres and in the space the original runs it in: a grid cell's
/// merged floor, origin at the cell centre (the edge rule's ±150/±300 cm
/// are the cell's quarters). `p` is the generator's parameter: the fine
/// columns are `p / 2` wide and the points, as in the original's
/// downsample, still 20 cm apart.
///
/// Points come in Z, then Y, then X order (X fastest), as the original
/// walks its fine blocks.
pub fn floor_points(bounds: Bounds, triangles: &[[Vec3; 3]], p: f32) -> Vec<Vec3> {
    let fine = p * FLOOR_FINE_SHARE;
    if fine <= 0.0 || fine.is_nan() || !fine.is_finite() {
        return Vec::new();
    }
    let centre = bounds.centre.to_array();
    let half = bounds.half_extents.to_array();
    // The low X edge, the high X edge, then every axis widened to the mesh.
    let mut low = [FLOOR_EDGES[0], -20.0, -300.0];
    let mut high = [FLOOR_EDGES[1], 0.0, -300.0];
    for &edge in FLOOR_EDGES.iter().take(4) {
        if centre[0] - half[0] < edge + 75.0 {
            low[0] = edge;
            break;
        }
    }
    for &edge in FLOOR_EDGES.iter().skip(1).rev() {
        if edge - 75.0 < centre[0] + half[0] {
            high[0] = edge;
            break;
        }
    }
    for axis in 0..3 {
        let lo = centre[axis] - half[axis];
        if lo < low[axis] {
            low[axis] = lo;
        }
    }
    for axis in 0..3 {
        let hi = centre[axis] + half[axis];
        if high[axis] <= hi && hi != high[axis] {
            high[axis] = hi;
        }
    }
    for axis in 0..3 {
        low[axis] = snap_edge(low[axis], 0.1);
        high[axis] = snap_edge(high[axis], -0.1);
    }
    low[1] -= 20.0;
    // The counts are kept as the original's 32-bit words.
    let dims = [0, 1, 2].map(|a| round_ties_even((high[a] - low[a]) / fine) as i32);
    if dims.iter().any(|&n| n <= 0) {
        return Vec::new();
    }
    let dims = dims.map(|n| n as usize);
    let Some(len) = dims
        .iter()
        .try_fold(1usize, |n, &d| n.checked_mul(d))
        .filter(|&n| n <= isize::MAX as usize)
    else {
        return Vec::new();
    };
    let mut marks = Vec::new();
    if marks.try_reserve_exact(len).is_err() {
        return Vec::new();
    }
    marks.resize(len, 0u8);
    let faces: Vec<SurfaceTriangle> = triangles
        .iter()
        .filter_map(|&t| SurfaceTriangle::new(t))
        .collect();
    let Some(columns) = face_columns(&faces, dims[0], dims[2], [low[0], low[2]], fine) else {
        return Vec::new();
    };
    let start_y = high[1] + fine;
    let max = (dims[1] as f32 + 1.0) * fine;
    let inverse = 1.0 / fine;
    for z in 0..dims[2] {
        let zc = (z as f32 + 0.5) * fine + low[2];
        for x in 0..dims[0] {
            let xc = (x as f32 + 0.5) * fine + low[0];
            let candidates = columns[x + z * dims[0]].iter().map(|&i| &faces[i as usize]);
            let Some(distance) =
                hit_among(candidates, Vec3::new(xc, start_y, zc), Vec3::NEG_Y, max)
            else {
                continue;
            };
            let y = dims[1] as i64 - round_ties_even(distance * inverse);
            // The original writes the mark unchecked; a face inside the
            // bounds always lands inside the grid.
            if (0..dims[1] as i64).contains(&y) {
                marks[x + y as usize * dims[0] + z * dims[0] * dims[1]] = 0xFF;
            }
        }
    }
    downsample(&marks, dims, low)
}

/// The faces each fine column's ray tests, in the faces' order.
///
/// A face tested on x and z can only take a ray that falls inside its x
/// and z range, so it joins only the columns over that range (one column
/// more on each side, far beyond rounding); a steep face, tested on y,
/// joins every column. Each ray then meets the same faces in the same
/// order as a test of the whole list, without the ones it cannot hit.
fn face_columns(
    faces: &[SurfaceTriangle],
    nx: usize,
    nz: usize,
    low: [f32; 2],
    fine: f32,
) -> Option<Vec<Vec<u32>>> {
    let count = nx.checked_mul(nz)?;
    let mut columns: Vec<Vec<u32>> = Vec::new();
    columns.try_reserve_exact(count).ok()?;
    columns.resize(count, Vec::new());
    // The columns whose centres lie in [lo, hi], widened by one.
    let span = |lo: f32, hi: f32, low: f32, n: usize| -> Option<(usize, usize)> {
        let first = ((lo - low) / fine - 0.5).floor() - 1.0;
        let last = ((hi - low) / fine - 0.5).ceil() + 1.0;
        if !(first.is_finite() && last.is_finite()) {
            return Some((0, n - 1));
        }
        if last < 0.0 || first > (n - 1) as f32 {
            return None;
        }
        Some((first.max(0.0) as usize, last.min((n - 1) as f32) as usize))
    };
    for (i, face) in faces.iter().enumerate() {
        let i = u32::try_from(i).ok()?;
        let (xs, zs) = match face.footprint {
            Some([x0, x1, z0, z1]) => {
                let (Some(xs), Some(zs)) = (span(x0, x1, low[0], nx), span(z0, z1, low[1], nz))
                else {
                    continue;
                };
                (xs, zs)
            }
            None => ((0, nx - 1), (0, nz - 1)),
        };
        for z in zs.0..=zs.1 {
            for x in xs.0..=xs.1 {
                columns[x + z * nx].push(i);
            }
        }
    }
    Some(columns)
}

/// The 2×2×2 downsample of the floor marks: a point on the hard-coded
/// 20 cm step for every block whose eight bytes sum past 1019.
///
/// The original walks the marks with a pointer that steps two rows per
/// block row and one slice per block slice; with an odd number of rows the
/// slices it reads drift one row back per block slice, and that is kept.
fn downsample(marks: &[u8], dims: [usize; 3], low: [f32; 3]) -> Vec<Vec3> {
    let [nx, ny, nz] = dims;
    let slice = nx * ny;
    let mut points = Vec::new();
    let mut base_slice = 0usize;
    for z in 0..nz / 2 {
        let mut row = base_slice;
        for y in 0..ny / 2 {
            for x in 0..nx / 2 {
                let at = row + x * 2;
                let sum = [
                    0,
                    1,
                    nx,
                    nx + 1,
                    slice,
                    slice + 1,
                    slice + nx,
                    slice + nx + 1,
                ]
                .iter()
                .map(|&d| marks[at + d] as u32)
                .sum::<u32>();
                if sum > FLOOR_MARK_SUM {
                    points.push(Vec3::new(
                        (x as f32 + 0.5) * FLOOR_POINT_STEP + low[0],
                        (y as f32 + 0.5) * FLOOR_POINT_STEP + low[1],
                        (z as f32 + 0.5) * FLOOR_POINT_STEP + low[2],
                    ));
                }
            }
            row += nx * 2;
        }
        base_slice = row + slice;
    }
    points
}

/// The share of the surface generator's parameter that one cell takes, a
/// double in the original.
const SURFACE_STEP_SHARE: f64 = 0.2;
/// The share of the step a cell's radius is when its centre touches the
/// surface: half a cell.
const SURFACE_CONTACT_SHARE: f32 = 0.5;
/// The point spacing the surface generator records on the node it fills
/// (no GI code reads it).
pub const SURFACE_POINT_SPACING: u32 = 40;
/// A cell with the surface in it.
const SURFACE_ON: u8 = 0xFF;
/// A cell with the surface within its larger sphere but not its own.
const SURFACE_NEAR_CELL: u8 = 0x80;
/// The weight a covered cell adds to a greedy cover's score.
const COVER_ON: f32 = 40.0;
/// The weight of an empty or out-of-grid cell, per step short of six.
const COVER_EMPTY: f32 = 4.0;
/// The weight of an already covered cell, per step short of six.
const COVER_TAKEN: f32 = 0.4;
/// The score a cover starts from: below every possible window.
const COVER_START: f32 = -1000.0;
/// The distance past which the greedy cover ignores a cell's penalty.
const COVER_REACH: f32 = 6.0;

/// How the surface generator turns its occupied cells into points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceCover {
    /// A greedy 5×5×5 cover, the original's default: each point stands for
    /// up to a 20 cm cube of surface.
    Greedy,
    /// One point at every occupied cell (the original's other builder
    /// flag; no shipped caller sets it).
    EveryCell,
}

/// The surface generator's cell counts per axis: `round(10 · half / p) + 3`
/// forced odd, one border cell on each side left out of the sampling.
fn surface_dims(bounds: Bounds, p: f32) -> [usize; 3] {
    [0, 1, 2].map(|axis| {
        let n = round_ties_even(bounds.half_extents[axis] * 10.0 / p);
        ((n as i32 as u32).wrapping_add(3) | 1) as usize
    })
}

/// The surface generator's cell size: `0.2 · p`, taken in double precision
/// and stored as a single.
fn surface_step(p: f32) -> f32 {
    (p as f64 * SURFACE_STEP_SHARE) as f32
}

/// The radius of the sphere that first asks whether the surface is anywhere
/// near a cell: half the cell's diagonal on a face, `s · √2 / 2`, in double
/// precision and stored as a single.
fn near_radius(step: f32) -> f32 {
    (step as f64 * std::f64::consts::SQRT_2 * 0.5) as f32
}

/// One cell of the surface generator's fine grid, in scan order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfaceCell {
    cell: [u16; 3],
    count: u16,
}

/// The general surface generator: a point for each part of the mesh's
/// surface, on a lattice a fifth of `p` wide (4 cm for the original's 20).
///
/// `bounds` is the mesh's own bounds and `p` the generator's parameter, in
/// the same unit. `inside` is the mesh's collision query: whether a sphere
/// of the given radius at a point touches the mesh. `cover` picks the
/// original's greedy cover or its one-point-per-cell variant.
///
/// A cell whose centre is within half a cell of the surface is "on"; one
/// whose centre only the larger near sphere reaches is "near". Near cells
/// next to an on cell are dropped, the rest become on, and the cover emits
/// its points in the order of the on cells' 3×3×3 counts, sparse first.
pub fn surface_points(
    bounds: Bounds,
    p: f32,
    inside: impl Fn(Vec3, f32) -> bool,
    cover: SurfaceCover,
) -> Vec<Vec3> {
    let step = surface_step(p);
    if step <= 0.0 || step.is_nan() || !step.is_finite() {
        return Vec::new();
    }
    let dims = surface_dims(bounds, p);
    if dims.iter().any(|&n| n < 3) {
        return Vec::new();
    }
    let Some(len) = dims
        .iter()
        .try_fold(1usize, |n, &d| n.checked_mul(d))
        .filter(|&n| n <= isize::MAX as usize)
    else {
        return Vec::new();
    };
    let mut grid = Vec::new();
    if grid.try_reserve_exact(len).is_err() {
        return Vec::new();
    }
    grid.resize(len, 0u8);
    let near = near_radius(step);
    let contact = step * SURFACE_CONTACT_SHARE;
    let centre = bounds.centre;
    for z in 1..dims[2] - 1 {
        for y in 1..dims[1] - 1 {
            for x in 1..dims[0] - 1 {
                let at = centre
                    + Vec3::new(
                        (x as f32 + 0.5 - dims[0] as f32 * 0.5) * step,
                        (y as f32 + 0.5 - dims[1] as f32 * 0.5) * step,
                        (z as f32 + 0.5 - dims[2] as f32 * 0.5) * step,
                    );
                if !inside(at, near) {
                    continue;
                }
                grid[x + y * dims[0] + z * dims[0] * dims[1]] = if inside(at, contact) {
                    SURFACE_ON
                } else {
                    SURFACE_NEAR_CELL
                };
            }
        }
    }
    let list = clean_and_list(&mut grid, dims);
    match cover {
        SurfaceCover::Greedy => greedy_points(&mut grid, dims, centre, step, &list),
        SurfaceCover::EveryCell => every_cell_points(&grid, dims, centre, step),
    }
}

/// The surface generator run once for a master node and shared by its
/// instances: the original voxelises the master and copies the point count
/// and buffer to every instance in its list.
pub fn shared_surface_points(
    bounds: Bounds,
    p: f32,
    inside: impl Fn(Vec3, f32) -> bool,
    cover: SurfaceCover,
) -> Arc<Vec<Vec3>> {
    Arc::new(surface_points(bounds, p, inside, cover))
}

/// The parameter every runtime call of the surface generator passes: 20,
/// in the centimetres the original's meshes and its generator work in, so
/// the sampling step is 4 cm.
pub const GENERATED_POINT_PARAMETER: f32 = 20.0;

/// The centimetres per metre the original's generators work in.
const CM_PER_M: f32 = 100.0;

/// The second flag word bits that make a floor-set child node get runtime
/// surface points: the original generates them when either bit `0x10000`
/// or bit `0x8` is set on the node, and for neither bit otherwise.
pub const GENERATED_POINT_FLAGS2: u32 = 0x0001_0008;

/// Whether a floor-set child node gets its surface points generated at
/// load: the original's floor-set walk runs the generator for a
/// class-`0x89` node whose second flag word has bit `0x10000` or bit `0x8`
/// set (the walk itself is `exa_assets`'s, from the centre-floor variants).
pub fn floor_set_node_generates(flags2: u32) -> bool {
    flags2 & GENERATED_POINT_FLAGS2 != 0
}

/// The surface generator run at load on a mesh that ships without a GI
/// point chunk, in the mesh's own space (metres).
///
/// The original's generator works in centimetres at
/// [`GENERATED_POINT_PARAMETER`]; Exarust's mesh positions and bounds are
/// metres, so the bounds and triangles are scaled up, sampled, and the
/// points scaled back. The scaling back is a guess: the RFC stores its
/// coordinates in centimetres and the reader divides by 100, so recovering
/// them is within a float ulp rather than bit for bit.
///
/// The point query is [`MeshQuery`] over the given triangles, the
/// stand-in for the node's collision query; see [`mesh_touches`]'s note.
pub fn generated_surface_points(
    bounds: Bounds,
    triangles: &[[Vec3; 3]],
    cover: SurfaceCover,
) -> Vec<Vec3> {
    let bounds = Bounds {
        centre: bounds.centre * CM_PER_M,
        half_extents: bounds.half_extents * CM_PER_M,
    };
    let scaled: Vec<[Vec3; 3]> = triangles.iter().map(|t| t.map(|p| p * CM_PER_M)).collect();
    let query = MeshQuery::new(&scaled);
    surface_points(
        bounds,
        GENERATED_POINT_PARAMETER,
        |point, radius| query.touches(point, radius),
        cover,
    )
    .into_iter()
    .map(|p| p / CM_PER_M)
    .collect()
}

/// Whether a sphere touches a mesh given as triangles: the query the
/// surface generator makes through a node's collision data.
///
/// The original's query is the node's own (a kd-tree over its collision
/// triangles), so this is the stand-in used here: the sphere touches when
/// its centre is no farther than its radius from any triangle's closest
/// point. Guess: the node's query is a triangle distance test, and it is
/// inclusive at the radius.
pub fn mesh_touches(triangles: &[[Vec3; 3]], centre: Vec3, radius: f32) -> bool {
    let limit = radius * radius;
    triangles
        .iter()
        .any(|t| closest_distance_squared(*t, centre) <= limit)
}

/// [`mesh_touches`] over a triangle list with a bounding-volume tree, for
/// the millions of queries one runtime generation makes.
///
/// The tree changes only the cost, never the answer: a node whose box is
/// farther than the radius is skipped, and a leaf tests its triangles with
/// the same closest-point predicate as [`mesh_touches`]. The original
/// reaches the same triangles through its node's kd-tree.
#[derive(Debug, Clone)]
pub struct MeshQuery {
    /// Leaves are `[start, end)` ranges into `order`.
    nodes: Vec<QueryNode>,
    order: Vec<u32>,
    triangles: Vec<[Vec3; 3]>,
}

#[derive(Debug, Clone)]
struct QueryNode {
    min: Vec3,
    max: Vec3,
    start: u32,
    end: u32,
    /// The two children's node indices; `None` for a leaf.
    children: Option<[u32; 2]>,
}

/// The most triangles a leaf keeps before the tree splits it.
const QUERY_LEAF: usize = 4;

impl MeshQuery {
    /// Builds the tree over `triangles`.
    pub fn new(triangles: &[[Vec3; 3]]) -> Self {
        let mut order: Vec<u32> = (0..triangles.len() as u32).collect();
        let mut nodes = Vec::new();
        if !triangles.is_empty() {
            let len = order.len();
            build_query_node(&mut nodes, &mut order, triangles, 0, len);
        }
        Self {
            nodes,
            order,
            triangles: triangles.to_vec(),
        }
    }

    /// Whether any triangle's closest point is within `radius` of `centre`.
    pub fn touches(&self, centre: Vec3, radius: f32) -> bool {
        if self.nodes.is_empty() {
            return false;
        }
        let limit = radius * radius;
        let mut stack = vec![0u32];
        while let Some(at) = stack.pop() {
            let Some(node) = self.nodes.get(at as usize) else {
                continue;
            };
            if box_distance_squared(node.min, node.max, centre) > limit {
                continue;
            }
            match node.children {
                Some(children) => stack.extend(children),
                None => {
                    for &i in &self.order[node.start as usize..node.end as usize] {
                        if let Some(t) = self.triangles.get(i as usize)
                            && closest_distance_squared(*t, centre) <= limit
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// Appends a node covering `order[start..end]` and returns its index.
fn build_query_node(
    nodes: &mut Vec<QueryNode>,
    order: &mut [u32],
    triangles: &[[Vec3; 3]],
    start: usize,
    end: usize,
) -> u32 {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for &i in &order[start..end] {
        if let Some(t) = triangles.get(i as usize) {
            for p in t {
                min = min.min(*p);
                max = max.max(*p);
            }
        }
    }
    let at = nodes.len() as u32;
    nodes.push(QueryNode {
        min,
        max,
        start: start as u32,
        end: end as u32,
        children: None,
    });
    if end - start <= QUERY_LEAF {
        return at;
    }
    let axis = {
        let d = max - min;
        if d.x >= d.y && d.x >= d.z {
            0
        } else if d.y >= d.z {
            1
        } else {
            2
        }
    };
    let mid = (start + end) / 2;
    order[start..end].select_nth_unstable_by(mid - start, |&a, &b| {
        let ca = triangle_centre(triangles, a)[axis];
        let cb = triangle_centre(triangles, b)[axis];
        ca.total_cmp(&cb)
    });
    let left = build_query_node(nodes, order, triangles, start, mid);
    let right = build_query_node(nodes, order, triangles, mid, end);
    nodes[at as usize].children = Some([left, right]);
    at
}

/// A triangle's centroid, for the tree's split.
fn triangle_centre(triangles: &[[Vec3; 3]], i: u32) -> Vec3 {
    match triangles.get(i as usize) {
        Some([a, b, c]) => (*a + *b + *c) / 3.0,
        None => Vec3::ZERO,
    }
}

/// The squared distance from `p` to a box, zero inside it.
fn box_distance_squared(min: Vec3, max: Vec3, p: Vec3) -> f32 {
    let outside = (min - p).max(p - max).max(Vec3::ZERO);
    outside.length_squared()
}

/// The squared distance from `p` to the closest point of a triangle.
fn closest_distance_squared(t: [Vec3; 3], p: Vec3) -> f32 {
    let [a, b, c] = t;
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return ap.length_squared();
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return bp.length_squared();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (p - (a + ab * v)).length_squared();
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return cp.length_squared();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (p - (a + ac * w)).length_squared();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (p - (b + (c - b) * w)).length_squared();
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (p - (a + ab * v + ac * w)).length_squared()
}

/// The pass before the cover: near cells with an on cell in their 3×3×3
/// are cleared, the rest become on, and all on cells are listed by their
/// 3×3×3 count, ascending, equal counts in scan order.
fn clean_and_list(grid: &mut [u8], dims: [usize; 3]) -> Vec<SurfaceCell> {
    let [nx, ny, nz] = dims;
    let index = |x: usize, y: usize, z: usize| x + y * nx + z * nx * ny;
    for z in 1..nz - 1 {
        for y in 1..ny - 1 {
            for x in 1..nx - 1 {
                if grid[index(x, y, z)] != SURFACE_NEAR_CELL {
                    continue;
                }
                'cells: for dz in -1i32..=1 {
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let at = index(
                                (x as i32 + dx) as usize,
                                (y as i32 + dy) as usize,
                                (z as i32 + dz) as usize,
                            );
                            if grid[at] == SURFACE_ON {
                                grid[index(x, y, z)] = 0;
                                break 'cells;
                            }
                        }
                    }
                }
            }
        }
    }
    let mut list: Vec<SurfaceCell> = Vec::new();
    for z in 1..nz - 1 {
        for y in 1..ny - 1 {
            for x in 1..nx - 1 {
                if grid[index(x, y, z)] == 0 {
                    continue;
                }
                grid[index(x, y, z)] = SURFACE_ON;
                let mut count = 0u16;
                for dz in -1i32..=1 {
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let at = index(
                                (x as i32 + dx) as usize,
                                (y as i32 + dy) as usize,
                                (z as i32 + dz) as usize,
                            );
                            if grid[at] != 0 {
                                count += 1;
                            }
                        }
                    }
                }
                let cell = [x as u16, y as u16, z as u16];
                list.push(SurfaceCell { cell, count });
            }
        }
    }
    // The original inserts each cell after every equal count (a binary
    // search, then a walk past equals): an ascending stable sort of the scan
    // order gives the same list without the quadratic shifting.
    list.sort_by_key(|c| c.count);
    list
}

/// Where a count goes in the ascending list: after every equal count, as
/// the original's binary search and its walk past equals do. The reference
/// the stable sort in [`clean_and_list`] is checked against.
#[cfg(test)]
fn list_position(list: &[SurfaceCell], count: u16) -> usize {
    let mut low = 0i64;
    let mut high = list.len() as i64 - 1;
    let mut mid = (high / 2) as usize;
    while low <= high {
        if (count as i32) < (list[mid].count as i32) {
            high = mid as i64 - 1;
        } else {
            if count as i32 <= list[mid].count as i32 {
                break;
            }
            low = mid as i64 + 1;
        }
        mid = ((high - low) / 2 + low) as usize;
    }
    while mid < list.len() && list[mid].count == count {
        mid += 1;
    }
    mid
}

/// A point at a cell's centre.
fn cell_point(centre: Vec3, step: f32, dims: [usize; 3], cell: [usize; 3]) -> Vec3 {
    centre
        + Vec3::new(
            (cell[0] as f32 + 0.5 - dims[0] as f32 * 0.5) * step,
            (cell[1] as f32 + 0.5 - dims[1] as f32 * 0.5) * step,
            (cell[2] as f32 + 0.5 - dims[2] as f32 * 0.5) * step,
        )
}

/// One point at every on cell, in scan order (Z, then Y, then X).
fn every_cell_points(grid: &[u8], dims: [usize; 3], centre: Vec3, step: f32) -> Vec<Vec3> {
    // The whole grid, borders included, as the original walks it.
    let [nx, ny, nz] = dims;
    let mut points = Vec::new();
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                if grid[x + y * nx + z * nx * ny] == SURFACE_ON {
                    points.push(cell_point(centre, step, dims, [x, y, z]));
                }
            }
        }
    }
    points
}

/// The greedy cover: for each listed on cell still on, the best-scoring
/// centre within ±2 cells of it takes a point and decrements every on cell
/// of its 5×5×5 window; the first centre of equal score wins, in Z, Y, X
/// order.
fn greedy_points(
    grid: &mut [u8],
    dims: [usize; 3],
    centre: Vec3,
    step: f32,
    list: &[SurfaceCell],
) -> Vec<Vec3> {
    let [nx, ny, nz] = dims;
    let index = |x: usize, y: usize, z: usize| x + y * nx + z * nx * ny;
    let mut points = Vec::new();
    let mut window: Vec<usize> = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    for entry in list {
        let [x, y, z] = entry.cell.map(usize::from);
        if grid[index(x, y, z)] != SURFACE_ON {
            continue;
        }
        let (zl, zh) = (z.saturating_sub(2), (z + 2).min(nz - 1));
        let (yl, yh) = (y.saturating_sub(2), (y + 2).min(ny - 1));
        let (xl, xh) = (x.saturating_sub(2), (x + 2).min(nx - 1));
        let mut best_score = COVER_START;
        let mut best = None;
        for zq in zl..=zh {
            for yq in yl..=yh {
                for xq in xl..=xh {
                    let mut score = 0.0f32;
                    window.clear();
                    for dz in -2i32..=2 {
                        for dy in -2i32..=2 {
                            for dx in -2i32..=2 {
                                let (cx, cy, cz) = (xq as i32 + dx, yq as i32 + dy, zq as i32 + dz);
                                let distance = (dx.abs() + dy.abs() + dz.abs()) as f32;
                                if cx < 0
                                    || cy < 0
                                    || cz < 0
                                    || cx >= nx as i32
                                    || cy >= ny as i32
                                    || cz >= nz as i32
                                {
                                    if distance < COVER_REACH {
                                        score -= (COVER_REACH - distance) * COVER_EMPTY;
                                    }
                                    continue;
                                }
                                let at = index(cx as usize, cy as usize, cz as usize);
                                match grid[at] {
                                    SURFACE_ON => {
                                        score += COVER_ON;
                                        window.push(at);
                                    }
                                    0 => {
                                        if distance < COVER_REACH {
                                            score -= (COVER_REACH - distance) * COVER_EMPTY;
                                        }
                                    }
                                    _ => {
                                        if distance < COVER_REACH {
                                            score -= (COVER_REACH - distance) * COVER_TAKEN;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if best_score < score {
                        best_score = score;
                        best = Some([xq, yq, zq]);
                        taken.clone_from(&window);
                    }
                }
            }
        }
        let Some(at) = best else { continue };
        if taken.is_empty() {
            continue;
        }
        points.push(cell_point(centre, step, dims, at));
        for &cell in &taken {
            if grid[cell] != 0 {
                grid[cell] -= 1;
            }
        }
    }
    points
}

#[cfg(test)]
mod tests;
