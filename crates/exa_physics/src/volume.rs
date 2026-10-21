//! The dynamics volumes' voxel summary and the mesh voxeliser the rigid-body
//! shape builders use (the original's `RF_Dynamics_Volumes`).
//!
//! A body built from a mesh is fitted by voxelising it ([`fit`]): a bordered
//! box over the mesh's bounds is cut into cells, cells near the surface are
//! marked by a sphere query, the cells inside are found by casting rays
//! along all three axes (a cell is solid when both rays of every axis hit),
//! the solid cells give the volume and the centre of mass ([`summarise`]),
//! an 8-bit corner mask per cell describes the voxel surface, and spheres
//! are extracted from the solid cells and their corners.
//!
//! The volume weighs the body: its mobility is 10 000 over the volume, and
//! its four point masses are 4 over the barycentric weights of the centre of
//! mass on its rest tetrahedron ([`VolumeSummary::mobility`],
//! [`VolumeSummary::point_masses`]). There is no buoyancy or drag in the
//! original's physics: the only velocity damping is the environment's
//! retention and the cloth's drag, so none is modelled here.
//!
//! Lengths are the original's centimetres, as everywhere the body links
//! are, and the spheres come out in the mesh's own space.

use bevy::math::{Mat3, Vec3};

use crate::voxel::{Bounds, Sphere};

/// A volume's centre and mass. The centre is an offset from the grid box's
/// centre (the bounds' centre), the arithmetic the original's summary
/// returns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeSummary {
    pub centre: [f32; 3],
    pub mass: f32,
}

/// The mass returned for a volume with no filled cell (0.1).
pub const EMPTY_MASS: f32 = 0.1;

/// The volume a body of mobility 1 has: mobility = 10 000 / volume.
pub const MOBILITY_VOLUME: f32 = 10_000.0;

/// The resolution the builders ask for, per cm of the mesh's longest half
/// extent, when the smallest half extent is under 10 cm.
pub const COARSE_RESOLUTION: f32 = 0.6;
/// The resolution when the smallest half extent is at least 10 cm.
pub const FINE_RESOLUTION: f32 = 0.3;
/// The smallest half extent that switches to the fine resolution.
pub const FINE_HALF_EXTENT: f32 = 10.0;
/// The resolution with the global high-quality flag: one cell per cm.
pub const PRECISE_RESOLUTION: f32 = 1.0;
/// The fewest cells along the longest axis.
pub const MIN_DEPTH: i32 = 6;
/// The fewest cells along the longest axis with the high-quality flag.
pub const PRECISE_MIN_DEPTH: i32 = 8;
/// The most cells along the longest axis.
pub const MAX_DEPTH: i32 = 96;
/// The cell offset scale the corner spheres use: the map's default
/// resolution (0.02) times the cell size.
pub const CORNER_OFFSET_SCALE: f32 = 0.02;

impl VolumeSummary {
    /// The mobility a body of this volume has (10 000 / volume).
    pub fn mobility(&self) -> f32 {
        MOBILITY_VOLUME / self.mass
    }

    /// The four point masses of a rigid body with rest tetrahedron `rest`
    /// whose centre of mass is this summary's: 4 over each vertex's
    /// barycentric weight of the centre, so the gravity shares put gravity
    /// at the centre of mass. An empty volume (mass 0.1), a flat
    /// tetrahedron and a centre on a face give equal masses.
    pub fn point_masses(&self, rest: [Vec3; 4]) -> [f32; 4] {
        if self.mass == EMPTY_MASS {
            return [4.0; 4];
        }
        let edges = Mat3::from_cols(rest[1] - rest[0], rest[2] - rest[0], rest[3] - rest[0]);
        let centre = Vec3::from(self.centre);
        if edges.determinant().abs() <= f32::EPSILON || !centre.is_finite() {
            return [4.0; 4];
        }
        let w = edges.inverse() * (centre - rest[0]);
        let masses = [
            4.0 / (1.0 - w.x - w.y - w.z),
            4.0 / w.x,
            4.0 / w.y,
            4.0 / w.z,
        ];
        if masses.iter().all(|m| m.is_finite()) {
            masses
        } else {
            [4.0; 4]
        }
    }
}

/// Sorts entries by their float key with the original's quicksort: a middle
/// pivot, values below it from the left and above it from the right, then
/// both sides (the right one by looping, as the original recurses on the
/// left and continues on the right).
pub fn sort_by_key(entries: &mut [(f32, u32)]) {
    if entries.len() > 1 {
        quicksort(entries, 0, entries.len() as isize - 1);
    }
}

fn quicksort(entries: &mut [(f32, u32)], low: isize, high: isize) {
    let mut i = low;
    let mut j = high;
    let pivot = entries[((low + high) / 2) as usize].0;
    loop {
        while entries[i as usize].0 < pivot {
            i += 1;
        }
        while entries[j as usize].0 > pivot {
            j -= 1;
        }
        if i <= j {
            entries.swap(i as usize, j as usize);
            i += 1;
            j -= 1;
        }
        if i > j {
            break;
        }
    }
    if low < j {
        quicksort(entries, low, j);
    }
    if i < high {
        quicksort(entries, i, high);
    }
}

/// Summarises a voxel volume. `dims` is the unpadded cell count per axis;
/// the count grid is padded by one cell on each side, so it holds
/// `(dims + 2)` cells per axis. Each filled cell contributes its centre
/// `(i, j, k) - 0.5` to the centre sum and one to the count; the centre is
/// `(sum/count/dim - 0.5)·scale` and the mass is the count times the cell
/// volume `2·scale_z/dim_z · (2·scale_y/dim_y · 2·scale_x/dim_x)`, in the
/// original's summation order. A volume with any axis under one cell, or no
/// filled cell, has no centre and the fallback mass [`EMPTY_MASS`].
pub fn summarise(dims: [i32; 3], scale: [f32; 3], counts: &[i32]) -> VolumeSummary {
    if dims.iter().any(|&d| d < 1) {
        return VolumeSummary {
            centre: [0.0; 3],
            mass: EMPTY_MASS,
        };
    }
    let stride_y = (dims[2] + 2) as usize;
    let stride_x = stride_y * (dims[1] + 2) as usize;
    let mut sum = [0.0f32; 3];
    let mut filled = 0i32;
    for i in 0..=dims[0] {
        for j in 0..=dims[1] {
            for k in 0..=dims[2] {
                let index = i as usize * stride_x + j as usize * stride_y + k as usize;
                if counts.get(index).copied().unwrap_or(0) > 0 {
                    // The original adds the index to the running sum, then
                    // takes the half off: ((i + sum) - 0.5).
                    sum[0] = (i as f32 + sum[0]) - 0.5;
                    sum[1] = (j as f32 + sum[1]) - 0.5;
                    sum[2] = (k as f32 + sum[2]) - 0.5;
                    filled += 1;
                }
            }
        }
    }
    if filled == 0 {
        return VolumeSummary {
            centre: [0.0; 3],
            mass: EMPTY_MASS,
        };
    }
    let count = filled as f32;
    let centre = [
        (sum[0] / count / dims[0] as f32 - 0.5) * scale[0],
        (sum[1] / count / dims[1] as f32 - 0.5) * scale[1],
        (sum[2] / count / dims[2] as f32 - 0.5) * scale[2],
    ];
    // ((2·z/nz) · ((2·y/ny) · (2·x/nx))) · count, as the original multiplies.
    let x = scale[0] * 2.0 / dims[0] as f32;
    let y = scale[1] * 2.0 / dims[1] as f32;
    let z = scale[2] * 2.0 / dims[2] as f32;
    VolumeSummary {
        centre,
        mass: count * (z * (y * x)),
    }
}

/// How the shape builders make a body's voxel grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeOptions {
    /// Cells per cm of the longest half extent.
    pub resolution: f32,
    /// The fewest cells along the longest axis.
    pub min_depth: i32,
    /// Whether the other axes are made even too (the builders pass true).
    pub even_axes: bool,
    /// The fewest occupied corners a cell needs to enter the sphere fit
    /// (the builders pass 1).
    pub min_corners: i32,
    /// The fewest corners still uncovered a cell needs to give a corner
    /// sphere (the builders pass 0 for limbs).
    pub min_corner_corners: i32,
    /// The global high-quality flag: one cell per cm, at least eight cells,
    /// and the wider near-surface queries.
    pub precise: bool,
}

impl Default for VolumeOptions {
    fn default() -> Self {
        Self {
            resolution: FINE_RESOLUTION,
            min_depth: MIN_DEPTH,
            even_axes: true,
            min_corners: 1,
            min_corner_corners: 0,
            precise: false,
        }
    }
}

impl VolumeOptions {
    /// The options the builders choose for a mesh with these bounds: the
    /// fine resolution when its smallest half extent is at least 10 cm,
    /// else the coarse one; with `precise` one cell per cm and eight cells
    /// at least (the original's global quality flag).
    pub fn for_bounds(bounds: &Bounds, precise: bool) -> Self {
        if precise {
            return Self {
                resolution: PRECISE_RESOLUTION,
                min_depth: PRECISE_MIN_DEPTH,
                precise,
                ..Self::default()
            };
        }
        let smallest = bounds.half_extents.min_element();
        Self {
            resolution: if smallest >= FINE_HALF_EXTENT {
                FINE_RESOLUTION
            } else {
                COARSE_RESOLUTION
            },
            ..Self::default()
        }
    }
}

/// A mesh to fit a volume to: its triangles with the bounding box and the
/// bounding-volume tree the original's kd-tree gives it, so the grid's
/// ray and sphere queries do not test every triangle of a large mesh.
#[derive(Debug, Clone)]
pub struct Mesh {
    triangles: Vec<[Vec3; 3]>,
    bounds: Bounds,
    /// Bounding boxes of the tree's nodes, depth first: a leaf holds
    /// `count` triangles from `first`, an inner node two children at
    /// `left` and `left + 1`.
    nodes: Vec<Node>,
    order: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    lo: Vec3,
    hi: Vec3,
    left: u32,
    right: u32,
    count: u32,
    first: u32,
}

impl Mesh {
    /// A mesh's tree, its box the union of its triangles' boxes (the
    /// original's mesh kd-tree box). An empty mesh has no box or tree.
    pub fn new(triangles: &[[Vec3; 3]]) -> Self {
        let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
        for t in triangles {
            for v in t {
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
        }
        if triangles.is_empty() {
            lo = Vec3::ZERO;
            hi = Vec3::ZERO;
        }
        let bounds = Bounds {
            centre: (lo + hi) * 0.5,
            half_extents: (hi - lo) * 0.5,
        };
        let mut order: Vec<u32> = (0..triangles.len() as u32).collect();
        let mut nodes = Vec::new();
        if !triangles.is_empty() {
            let len = order.len();
            build_node(triangles, &mut order, 0, len, &mut nodes);
        }
        Self {
            triangles: triangles.to_vec(),
            bounds,
            nodes,
            order,
        }
    }

    /// The box around every triangle.
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    pub fn triangles(&self) -> &[[Vec3; 3]] {
        &self.triangles
    }

    /// Whether any triangle's closest point is within `radius` of
    /// `centre`.
    pub fn touches(&self, centre: Vec3, radius: f32) -> bool {
        let limit = radius * radius;
        let mut stack = vec![0u32];
        while let Some(index) = stack.pop() {
            let Some(node) = self.nodes.get(index as usize) else {
                continue;
            };
            if box_distance2(centre, node.lo, node.hi) > limit {
                continue;
            }
            if node.count > 0 {
                let first = node.first as usize;
                for &i in &self.order[first..first + node.count as usize] {
                    if let Some(t) = self.triangles.get(i as usize)
                        && crate::voxel::closest_distance_squared(*t, centre) <= limit
                    {
                        return true;
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        false
    }

    /// Whether a triangle is hit by the ray from `origin` along `direction`
    /// (unit) within `reach`.
    pub fn ray(&self, origin: Vec3, direction: Vec3, reach: f32) -> bool {
        let mut stack = vec![0u32];
        while let Some(index) = stack.pop() {
            let Some(node) = self.nodes.get(index as usize) else {
                continue;
            };
            if !segment_box(origin, direction, reach, node.lo, node.hi) {
                continue;
            }
            if node.count > 0 {
                let first = node.first as usize;
                for &i in &self.order[first..first + node.count as usize] {
                    if let Some(t) = self.triangles.get(i as usize)
                        && ray_hits_triangle(*t, origin, direction, reach)
                    {
                        return true;
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        false
    }
}

/// Builds one tree node over `order[start..start + count]`, splitting the
/// widest axis at the median of the triangles' centres, with at most eight
/// per leaf.
fn build_node(
    triangles: &[[Vec3; 3]],
    order: &mut [u32],
    start: usize,
    count: usize,
    nodes: &mut Vec<Node>,
) -> u32 {
    const LEAF: usize = 8;
    let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
    let (mut clo, mut chi) = (Vec3::MAX, Vec3::MIN);
    for &i in &order[start..start + count] {
        let t = triangles[i as usize];
        let c = (t[0] + t[1] + t[2]) / 3.0;
        clo = clo.min(c);
        chi = chi.max(c);
        lo = lo.min(t[0].min(t[1]).min(t[2]));
        hi = hi.max(t[0].max(t[1]).max(t[2]));
    }
    let index = nodes.len() as u32;
    nodes.push(Node {
        lo,
        hi,
        left: 0,
        right: 0,
        count: count as u32,
        first: start as u32,
    });
    if count <= LEAF {
        return index;
    }
    let extent = chi - clo;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    let range = &mut order[start..start + count];
    range.sort_by(|&a, &b| {
        let ca =
            (triangles[a as usize][0] + triangles[a as usize][1] + triangles[a as usize][2]) / 3.0;
        let cb =
            (triangles[b as usize][0] + triangles[b as usize][1] + triangles[b as usize][2]) / 3.0;
        ca[axis].total_cmp(&cb[axis])
    });
    let middle = count / 2;
    let left_index = build_node(triangles, order, start, middle, nodes);
    let right_index = build_node(triangles, order, start + middle, count - middle, nodes);
    if let Some(node) = nodes.get_mut(index as usize) {
        node.count = 0;
        node.left = left_index;
        node.right = right_index;
    }
    index
}

/// Squared distance from a point to an axis-aligned box (0 inside).
fn box_distance2(p: Vec3, lo: Vec3, hi: Vec3) -> f32 {
    let d = (lo - p).max(Vec3::ZERO).max(p - hi);
    d.length_squared()
}

/// Whether the segment `origin .. origin + direction·reach` meets the box.
fn segment_box(origin: Vec3, direction: Vec3, reach: f32, lo: Vec3, hi: Vec3) -> bool {
    let mut near = 0.0f32;
    let mut far = reach;
    for a in 0..3 {
        let d = direction[a];
        if d.abs() < 1e-12 {
            if origin[a] < lo[a] || origin[a] > hi[a] {
                return false;
            }
            continue;
        }
        let inv = 1.0 / d;
        let (mut t0, mut t1) = ((lo[a] - origin[a]) * inv, (hi[a] - origin[a]) * inv);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        near = near.max(t0);
        far = far.min(t1);
        if near > far {
            return false;
        }
    }
    true
}

/// Whether the ray from `origin` along `direction` (unit) meets the
/// triangle within `reach`, two-sided.
fn ray_hits_triangle(t: [Vec3; 3], origin: Vec3, direction: Vec3, reach: f32) -> bool {
    let [a, b, c] = t;
    let e1 = b - a;
    let e2 = c - a;
    let p = direction.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return false;
    }
    let inv = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = s.cross(e1);
    let v = direction.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let distance = e2.dot(q) * inv;
    distance >= 0.0 && distance <= reach
}

/// A fitted voxel volume: the grid's cell counts per axis, its volume and
/// centre, and the collision spheres in the mesh's space.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedVolume {
    /// Cells per axis of the grid's box, without the border.
    pub counts: [i32; 3],
    pub summary: VolumeSummary,
    pub spheres: Vec<Sphere>,
}

/// The original's rigid-body voxeliser: the grid fill, the solid fill, the
/// volume and centre, the corner masks and the sphere extraction, in the
/// order the shape builders run them. The mesh's triangles must be in the
/// original's centimetres.
pub fn fit(mesh: &Mesh, options: &VolumeOptions) -> FittedVolume {
    if mesh.triangles.is_empty() {
        return FittedVolume {
            counts: [0; 3],
            summary: summarise([0; 3], [0.0; 3], &[]),
            spheres: Vec::new(),
        };
    }
    let mut grid = Grid::new(mesh.bounds, options);
    grid.mark_surface(mesh, options);
    grid.mark_solid(mesh);
    let summary = summarise(grid.counts, grid.half.to_array(), &grid.cells);
    grid.mark_corners();
    let spheres = grid.extract_spheres(options);
    FittedVolume {
        counts: grid.counts,
        summary,
        spheres,
    }
}

/// One cell's corner, as a cell offset: bit 0 is the cell itself, bit 1
/// its +z neighbour, bit 2 its +y, bit 4 its +x (the original's mask order).
const MASK_CELLS: [[i32; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 0],
    [0, 1, 1],
    [1, 0, 0],
    [1, 0, 1],
    [1, 1, 0],
    [1, 1, 1],
];

/// The corner a mask bit stands for, as an offset from the cell's centre in
/// half cells: the original's corner table for the sphere offsets.
const CORNER_OFFSETS: [[i32; 3]; 8] = [
    [-1, -1, -1],
    [-1, -1, 1],
    [-1, 1, -1],
    [-1, 1, 1],
    [1, -1, -1],
    [1, -1, 1],
    [1, 1, -1],
    [1, 1, 1],
];

/// The 26 neighbours, in the original's table order: x, then y, then z.
const NEIGHBOURS: [[i32; 3]; 26] = {
    let mut out = [[0i32; 3]; 26];
    let mut n = 0;
    let mut x = -1;
    while x <= 1 {
        let mut y = -1;
        while y <= 1 {
            let mut z = -1;
            while z <= 1 {
                if x != 0 || y != 0 || z != 0 {
                    out[n] = [x, y, z];
                    n += 1;
                }
                z += 1;
            }
            y += 1;
        }
        x += 1;
    }
    out
};

/// The six face neighbours, in the original's order: -x, +x, -y, +y, -z, +z.
const FACES: [[i32; 3]; 6] = [
    [-1, 0, 0],
    [1, 0, 0],
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
];

/// The bits of a neighbour's corner mask that map onto this cell's corners,
/// by the neighbour's offset: the original's three 16-bit AND tables. A
/// neighbour's bit index is `4·x + 2·y + z`, so the mask is rotated by
/// `-4·dx - 2·dy - dz`; the tables keep only the corners that coincide.
fn corner_shift(dx: i32, dy: i32, dz: i32) -> (i32, u32) {
    // The shift amount, read from the low words at +4/+0x10/+0x1c of the
    // original's tables: (-4, 0, 4), (-2, 0, 2), (-1, 0, 1).
    let shift = -4 * dx - 2 * dy - dz;
    let x = match dx {
        -1 => 0x000f,
        1 => 0xfffe,
        _ => 0,
    };
    let y = match dy {
        -1 => 0x0033,
        1 => 0xcc00,
        _ => 0x00ff,
    };
    let z = match dz {
        -1 => 0x0055,
        1 => 0x0000,
        _ => 0x00aa,
    };
    (shift, x & y & z)
}

/// Rotates an 8-bit corner mask left by 4, then 2, then 1 bit (the
/// original's three rotation tables), i.e. right by one.
fn rotate_mask(mask: u8) -> u8 {
    let m = mask as u32;
    let r = ((m & 0x000f) << 4) | ((m & 0xfffe) >> 4);
    let r = ((r & 0x0033) << 2) | ((r & 0x00cc) >> 2);
    let r = ((r & 0x0055) << 1) | ((r & 0x00aa) >> 1);
    (r & 0xff) as u8
}

/// The voxel grid of one fit: a bordered box of cells, one word each.
struct Grid {
    /// Centre and half extents of the box.
    centre: Vec3,
    half: Vec3,
    /// Cells per axis without the border.
    counts: [i32; 3],
    /// The box's low corner (min − half), per axis.
    low: [f32; 3],
    /// One cell's size per axis.
    step: [f32; 3],
    cells: Vec<i32>,
    fill: Vec<[u8; 2]>,
    /// The padded cell count per axis (the allocation's dims).
    dims: [usize; 3],
}

impl Grid {
    /// The grid the original's fill makes: the longest axis gets
    /// round(half·resolution) cells made even, at least the minimum and at
    /// most 96; the other axes the floored proportion (made even when asked).
    /// A longest half extent under 1 cm makes no grid.
    fn new(bounds: Bounds, options: &VolumeOptions) -> Self {
        let mut counts = [0i32; 3];
        let mut axis = 0;
        let mut longest = bounds.half_extents.x;
        for i in 1..3 {
            // The original takes a new longest on a tie and on NaN.
            if longest < bounds.half_extents[i] || longest.is_nan() {
                longest = bounds.half_extents[i];
                axis = i;
            }
        }
        if longest >= 1.0 {
            // The longest axis rounds to even (CVTSS2SI), then is made even,
            // clamped to the minimum and 96.
            let mut n = round_half_even(longest * options.resolution);
            if n % 2 != 0 {
                n += 1;
            }
            n = n.max(options.min_depth).min(MAX_DEPTH);
            counts[axis] = n;
            for (i, count) in counts.iter_mut().enumerate() {
                if i == axis {
                    continue;
                }
                // The original floors these proportions (its round helper
                // rounds towards minus infinity).
                let mut m = ratio_floor(n, bounds.half_extents[i], longest);
                if options.even_axes && m % 2 != 0 {
                    m += 1;
                }
                *count = m;
            }
            // The original clamps a count under 1 after its first pass;
            // here it is clamped before the grid is made, so a mesh thinner
            // than a sixth of its longest axis cannot overrun its own grid
            // (no shipped mesh comes near; see the README).
            for c in &mut counts {
                *c = (*c).max(1);
            }
        }
        let dims = counts.map(|c| (c + 2).max(0) as usize);
        let len = dims[0] * dims[1] * dims[2];
        let step = [
            bounds.half_extents.x * 2.0 / counts[0].max(1) as f32,
            bounds.half_extents.y * 2.0 / counts[1].max(1) as f32,
            bounds.half_extents.z * 2.0 / counts[2].max(1) as f32,
        ];
        // The border cell's centre lies half a cell outside the box: the
        // low cell of the allocation starts at min − half/count.
        let low = [
            (bounds.centre.x - bounds.half_extents.x)
                - bounds.half_extents.x / counts[0].max(1) as f32,
            (bounds.centre.y - bounds.half_extents.y)
                - bounds.half_extents.y / counts[1].max(1) as f32,
            (bounds.centre.z - bounds.half_extents.z)
                - bounds.half_extents.z / counts[2].max(1) as f32,
        ];
        Self {
            centre: bounds.centre,
            half: bounds.half_extents,
            counts,
            low,
            step,
            cells: vec![0; len],
            fill: vec![[0; 2]; len],
            dims,
        }
    }

    fn index(&self, cell: [i32; 3]) -> Option<usize> {
        let [nx, ny, nz] = self.dims;
        if cell[0] < 0
            || cell[1] < 0
            || cell[2] < 0
            || cell[0] as usize >= nx
            || cell[1] as usize >= ny
            || cell[2] as usize >= nz
            || self.cells.is_empty()
        {
            return None;
        }
        Some(cell[2] as usize + nz * (cell[1] as usize + ny * cell[0] as usize))
    }

    fn cell_centre(&self, cell: [i32; 3]) -> Vec3 {
        Vec3::new(
            cell[0] as f32 * self.step[0] + self.low[0],
            cell[1] as f32 * self.step[1] + self.low[1],
            cell[2] as f32 * self.step[2] + self.low[2],
        )
    }

    fn word(&self, cell: [i32; 3]) -> i32 {
        self.index(cell)
            .and_then(|i| self.cells.get(i))
            .copied()
            .unwrap_or(0)
    }

    /// Whether the cell holds a surface or a solid (any non-zero word).
    fn filled(&self, cell: [i32; 3]) -> bool {
        self.word(cell) != 0
    }

    /// The original's grid fill: cells whose centre lies within half the
    /// longest axis's cell (a quarter with the precise flag) of the mesh's
    /// surface are marked near; a cell away from the surface with more than
    /// four near neighbours (their offsets summing to at most three per
    /// axis) whose centre touches the mesh within 0.75·√2 half-cells is
    /// marked provisional; every provisional cell becomes solid.
    fn mark_surface(&mut self, mesh: &Mesh, options: &VolumeOptions) {
        if self.cells.is_empty() {
            return;
        }
        let axis = self.longest_axis();
        let half_cell = self.half[axis] / self.counts[axis].max(1) as f32;
        let near = if options.precise {
            half_cell * std::f32::consts::SQRT_2 * 0.5
        } else {
            half_cell
        };
        for i in 1..=self.counts[0] {
            for j in 1..=self.counts[1] {
                for k in 1..=self.counts[2] {
                    let at = [i, j, k];
                    if mesh.touches(self.cell_centre(at), near)
                        && let Some(n) = self.index(at)
                    {
                        self.cells[n] = 1;
                    }
                }
            }
        }
        let inner = if options.precise {
            half_cell * std::f32::consts::SQRT_2 * 0.75
        } else {
            half_cell
        };
        for i in 1..=self.counts[0] {
            for j in 1..=self.counts[1] {
                for k in 1..=self.counts[2] {
                    let at = [i, j, k];
                    if self.word(at) != 0 {
                        continue;
                    }
                    let (mut n, mut sum) = (0, [0i32; 3]);
                    for off in NEIGHBOURS {
                        let nb = [i + off[0], j + off[1], k + off[2]];
                        if self.word(nb) == 1 {
                            n += 1;
                            sum[0] += off[0];
                            sum[1] += off[1];
                            sum[2] += off[2];
                        }
                    }
                    if n > 4
                        && sum.iter().all(|s| s.abs() <= 3)
                        && mesh.touches(self.cell_centre(at), inner)
                        && let Some(index) = self.index(at)
                    {
                        self.cells[index] = 2;
                    }
                }
            }
        }
        for word in &mut self.cells {
            if *word == 2 {
                *word = 1;
            }
        }
    }

    fn longest_axis(&self) -> usize {
        let mut axis = 0;
        for i in 1..3 {
            if self.half[axis] < self.half[i] || self.half[axis].is_nan() {
                axis = i;
            }
        }
        axis
    }

    /// The original's solid fill: a cell still empty is solid when the mesh
    /// is hit by both rays of every axis from its centre. Returns the solid
    /// cell count.
    fn mark_solid(&mut self, mesh: &Mesh) -> i32 {
        if self.cells.is_empty() {
            return 0;
        }
        let min = self.centre - self.half;
        let max = self.centre + self.half;
        let mut solid = 0;
        for i in 1..=self.counts[0] {
            for j in 1..=self.counts[1] {
                for k in 1..=self.counts[2] {
                    let at = [i, j, k];
                    let size = self.word(at);
                    if size != 0 {
                        solid += 1;
                        continue;
                    }
                    let at3 = self.cell_centre(at);
                    let mut axes = 0;
                    for axis in 0..3 {
                        let mut hit = true;
                        for sign in [-1.0f32, 1.0] {
                            let dir = match axis {
                                0 => Vec3::new(sign, 0.0, 0.0),
                                1 => Vec3::new(0.0, sign, 0.0),
                                _ => Vec3::new(0.0, 0.0, sign),
                            };
                            // The ray reaches just past the box's face (the
                            // original's 0.2 cm margin).
                            let reach = if sign > 0.0 {
                                (max[axis] - at3[axis]) + 0.2
                            } else {
                                (at3[axis] - min[axis]) + 0.2
                            };
                            if reach < 0.0 || !mesh.ray(at3, dir, reach) {
                                hit = false;
                                break;
                            }
                        }
                        if hit {
                            axes += 1;
                            if axes > 2 {
                                break;
                            }
                        }
                    }
                    if axes > 2
                        && let Some(index) = self.index(at)
                    {
                        self.cells[index] = 1;
                        solid += 1;
                    }
                }
            }
        }
        solid
    }

    /// The original's corner masks: each cell counts how many of its eight
    /// corner cells (the cell itself and its +x/+y/+z neighbours) are
    /// filled, and keeps the matching 8-bit mask.
    fn mark_corners(&mut self) {
        if self.cells.is_empty() {
            return;
        }
        for i in 0..=self.counts[0] {
            for j in 0..=self.counts[1] {
                for k in 0..=self.counts[2] {
                    let mut count = 0u8;
                    let mut mask = 0u8;
                    for (bit, off) in MASK_CELLS.iter().enumerate() {
                        let at = [i + off[0], j + off[1], k + off[2]];
                        if self.filled(at) {
                            count += 1;
                            mask |= 1 << bit;
                        }
                    }
                    if count != 0
                        && let Some(index) = self.index([i, j, k])
                    {
                        self.fill[index] = [count, mask];
                    }
                }
            }
        }
    }
}

/// Rounds to the nearest integer, ties to even: the processor's CVTSS2SI,
/// which the original's inlined conversions use.
fn round_half_even(x: f32) -> i32 {
    let r = x.round_ties_even();
    if r > i32::MAX as f32 {
        i32::MAX
    } else if r < i32::MIN as f32 {
        i32::MIN
    } else {
        r as i32
    }
}

/// `floor(n · a / b)` the way the original's round helper does: it sets the
/// processor's rounding mode to round towards minus infinity and converts.
fn ratio_floor(n: i32, a: f32, b: f32) -> i32 {
    let value = n as f32 * a / b;
    if !value.is_finite() {
        return 0;
    }
    let f = value.floor();
    if f > i32::MAX as f32 {
        i32::MAX
    } else if f < i32::MIN as f32 {
        i32::MIN
    } else {
        f as i32
    }
}

/// Whether the ray from `origin` along `direction` (unit) meets a triangle
/// within `reach`: the original's kd-tree ray query, two-sided, tested
/// against every triangle.
pub fn ray_touches(triangles: &[[Vec3; 3]], origin: Vec3, direction: Vec3, reach: f32) -> bool {
    triangles
        .iter()
        .any(|&t| ray_hits_triangle(t, origin, direction, reach))
}

impl Grid {
    /// The original's sphere extraction: candidates are the interior solid
    /// cells (every occupied corner) with a radius to the nearest cell that
    /// is not full, at least one cell; each covers the cells within
    /// `r − 0.25`, the candidates are sorted by radius and any whose cells
    /// are all covered by another is dropped (taking its counts back), the
    /// survivors become spheres of `r/(1 + 0.01 r)` cells, and the cells on
    /// the voxel surface then give corner spheres. A mesh with no candidate
    /// at all takes the corner-only fallback.
    fn extract_spheres(&self, options: &VolumeOptions) -> Vec<Sphere> {
        if self.cells.is_empty() {
            return Vec::new();
        }
        let mut candidates: Vec<([i32; 3], f32)> = Vec::new();
        // The candidate search leaves out the outer cell layers: a cell's
        // radius is bounded by its distance to the grid's edge.
        let [nx, ny, nz] = self.dims;
        for i in 3..=self.counts[0] - 3 {
            for j in 3..=self.counts[1] - 3 {
                for k in 3..=self.counts[2] - 3 {
                    let at = [i, j, k];
                    if i32::from(self.fill_at(at)) <= options.min_corners {
                        continue;
                    }
                    let edge = (nx as i32 - i)
                        .min(i)
                        .min(ny as i32 - j)
                        .min(j)
                        .min(nz as i32 - k)
                        .min(k)
                        .max(0);
                    let mut radius2 = (edge * edge) as f32;
                    for x in -edge..=edge {
                        for y in -edge..=edge {
                            for z in -edge..=edge {
                                if self.fill_at([i + x, j + y, k + z]) < 8 {
                                    let d2 = (x * x + y * y + z * z) as f32;
                                    if d2 < radius2 {
                                        radius2 = d2;
                                    }
                                }
                            }
                        }
                    }
                    if radius2 >= 1.0 {
                        candidates.push((at, radius2.sqrt()));
                    }
                }
            }
        }
        let mut spheres = Vec::new();
        if candidates.is_empty() {
            self.fallback_spheres(&mut spheres, options);
            return spheres;
        }
        // The original sorts the candidate records by radius with its own
        // quicksort, so equal radii keep its order.
        let mut keys: Vec<(f32, u32)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (c.1, i as u32))
            .collect();
        sort_by_key(&mut keys);
        let sorted: Vec<([i32; 3], f32)> = keys
            .iter()
            .filter_map(|&(_, i)| candidates.get(i as usize).copied())
            .collect();
        candidates = sorted;
        let mut coverage = vec![0i32; self.cells.len()];
        let cover = |cands: &[([i32; 3], f32)], coverage: &mut [i32], grow: bool| {
            for &(at, r) in cands {
                if r <= 0.0 || r.is_nan() {
                    continue;
                }
                let lim = round_half_even(r + 0.5);
                let limit = if grow { r + 0.5 } else { r - 0.25 };
                if limit < 0.0 {
                    continue;
                }
                let limit2 = limit * limit;
                for x in -lim..=lim {
                    for y in -lim..=lim {
                        for z in -lim..=lim {
                            let (dx, dy, dz) = (x as f32 - 0.5, y as f32 - 0.5, z as f32 - 0.5);
                            if dx * dx + dy * dy + dz * dz <= limit2
                                && let Some(n) = self.index([at[0] + x, at[1] + y, at[2] + z])
                            {
                                coverage[n] += if grow { 1 } else { -1 };
                            }
                        }
                    }
                }
            }
        };
        cover(&candidates, &mut coverage, true);
        // Redundancy pruning: a candidate every one of whose cells another
        // covers is dropped and its counts taken back.
        let mut alive = vec![true; candidates.len()];
        for (index, &(at, r)) in candidates.iter().enumerate() {
            let lim = round_half_even(r + 0.5);
            let t = r - 0.25;
            if t < 0.0 {
                continue;
            }
            let t2 = t * t;
            let mut all_covered = true;
            'cells: for x in -lim..=lim {
                for y in -lim..=lim {
                    for z in -lim..=lim {
                        let (dx, dy, dz) = (x as f32 - 0.5, y as f32 - 0.5, z as f32 - 0.5);
                        if dx * dx + dy * dy + dz * dz <= t2
                            && let Some(n) = self.index([at[0] + x, at[1] + y, at[2] + z])
                            && coverage[n] < 2
                        {
                            all_covered = false;
                            break 'cells;
                        }
                    }
                }
            }
            if all_covered {
                alive[index] = false;
                cover(&[(at, r)], &mut coverage, false);
            }
        }
        // The final pass counts the spheres' coverage again with r + 0.5;
        // the corner pass below sees those counts.
        let survivors: Vec<([i32; 3], f32)> = candidates
            .iter()
            .zip(&alive)
            .filter(|&(_, &a)| a)
            .map(|(&c, _)| c)
            .collect();
        cover(&survivors, &mut coverage, true);
        let cell = Vec3::from(self.step);
        let longest = self.longest_axis();
        let radius_cell = self.half[longest] / self.counts[longest].max(1) as f32 * 2.0;
        let min = self.centre - self.half;
        for &(at, r) in &survivors {
            let centre = Vec3::new(at[0] as f32, at[1] as f32, at[2] as f32) * cell + min;
            // In double precision, as the original evaluates it.
            let r64 = f64::from(r);
            let radius = (r64 / (r64 * 0.01 + 1.0) * f64::from(radius_cell)) as f32;
            spheres.push(Sphere { centre, radius });
        }
        self.corner_spheres(&coverage, &mut spheres, options);
        spheres
    }

    /// The original's corner spheres: for each surface cell whose corners
    /// are not all covered by the candidate spheres, a sphere of one cell's
    /// radius sits a small step (the corner offsets times
    /// [`CORNER_OFFSET_SCALE`] of half a cell) from the cell's centre toward
    /// the empty corners, unless a neighbour with more corners already owns
    /// them or the mask is a flat piece of surface repeated by a rotation or
    /// a face neighbour.
    fn corner_spheres(&self, coverage: &[i32], spheres: &mut Vec<Sphere>, options: &VolumeOptions) {
        let cell = Vec3::from(self.step);
        let longest = self.longest_axis();
        let half_cell = self.half[longest] / self.counts[longest].max(1) as f32;
        let radius = half_cell * 2.0;
        let offset_scale = half_cell * CORNER_OFFSET_SCALE;
        let min = self.centre - self.half;
        for i in 1..=self.counts[0] - 1 {
            for j in 1..=self.counts[1] - 1 {
                for k in 1..=self.counts[2] - 1 {
                    let at = [i, j, k];
                    let Some(index) = self.index(at) else {
                        continue;
                    };
                    let [count, mask] = self.fill[index];
                    let count = count as i32;
                    if count <= options.min_corners {
                        continue;
                    }
                    // The corners a candidate sphere does not already cover.
                    let mut keep = mask;
                    let mut remaining = count;
                    for (bit, off) in MASK_CELLS.iter().enumerate() {
                        let corner = [i + off[0], j + off[1], k + off[2]];
                        let covered = self
                            .index(corner)
                            .and_then(|n| coverage.get(n))
                            .copied()
                            .unwrap_or(0)
                            > 0;
                        if covered {
                            remaining -= 1;
                            keep &= !(1 << bit);
                        }
                    }
                    if remaining < 2 {
                        continue;
                    }
                    // A neighbour with two more corners explains the
                    // corners the two share.
                    for off in NEIGHBOURS {
                        let nb = [i + off[0], j + off[1], k + off[2]];
                        let Some(n) = self.index(nb) else { continue };
                        let ncount = self.fill[n][0] as i32;
                        if ncount - count > 1 {
                            let (shift, filter) = corner_shift(off[0], off[1], off[2]);
                            keep &= !shared_corners(self.fill[n][1], shift, filter);
                        }
                    }
                    let emit = if keep == 0 {
                        // Every corner is explained: a flat piece of surface
                        // is only given a sphere when its mask repeats under
                        // the rotation or at a face neighbour.
                        let rotated = rotate_mask(mask);
                        let same = mask == rotated
                            || FACES.iter().any(|off| {
                                self.index([i + off[0], j + off[1], k + off[2]])
                                    .map(|n| self.fill[n][1] == rotated)
                                    .unwrap_or(false)
                            });
                        (4..7).contains(&count) && same && count > options.min_corner_corners
                    } else {
                        popcount(keep) as i32 > options.min_corner_corners
                    };
                    if !emit {
                        continue;
                    }
                    let dir = self.corner_direction(at, mask);
                    let centre = Vec3::new(
                        i as f32 * cell.x + min.x + dir[0] as f32 * offset_scale,
                        j as f32 * cell.y + min.y + dir[1] as f32 * offset_scale,
                        k as f32 * cell.z + min.z + dir[2] as f32 * offset_scale,
                    );
                    spheres.push(Sphere { centre, radius });
                }
            }
        }
    }

    /// The original's fallback for a mesh too thin for any candidate cell:
    /// the corner spheres over every surface cell, with the same radius and
    /// offset as the corner pass.
    fn fallback_spheres(&self, spheres: &mut Vec<Sphere>, options: &VolumeOptions) {
        let cell = Vec3::from(self.step);
        let longest = self.longest_axis();
        let half_cell = self.half[longest] / self.counts[longest].max(1) as f32;
        let radius = half_cell * 2.0;
        let offset_scale = half_cell * CORNER_OFFSET_SCALE;
        let min = self.centre - self.half;
        for i in 1..=self.counts[0] - 1 {
            for j in 1..=self.counts[1] - 1 {
                for k in 1..=self.counts[2] - 1 {
                    let at = [i, j, k];
                    let Some(index) = self.index(at) else {
                        continue;
                    };
                    let [count, mask] = self.fill[index];
                    let count = count as i32;
                    if count <= options.min_corners {
                        continue;
                    }
                    let mut keep = mask;
                    for off in NEIGHBOURS {
                        let nb = [i + off[0], j + off[1], k + off[2]];
                        let Some(n) = self.index(nb) else { continue };
                        let ncount = self.fill[n][0] as i32;
                        if ncount - count > 1 {
                            let (shift, filter) = corner_shift(off[0], off[1], off[2]);
                            keep &= !shared_corners(self.fill[n][1], shift, filter);
                        }
                    }
                    let emit = if keep == 0 {
                        let rotated = rotate_mask(mask);
                        let same = mask == rotated
                            || FACES.iter().any(|off| {
                                self.index([i + off[0], j + off[1], k + off[2]])
                                    .map(|n| self.fill[n][1] == rotated)
                                    .unwrap_or(false)
                            });
                        (4..7).contains(&count) && same && count > options.min_corner_corners
                    } else {
                        popcount(keep) as i32 > options.min_corner_corners
                    };
                    if !emit {
                        continue;
                    }
                    let dir = self.corner_direction(at, mask);
                    let centre = Vec3::new(
                        i as f32 * cell.x + min.x + dir[0] as f32 * offset_scale,
                        j as f32 * cell.y + min.y + dir[1] as f32 * offset_scale,
                        k as f32 * cell.z + min.z + dir[2] as f32 * offset_scale,
                    );
                    spheres.push(Sphere { centre, radius });
                }
            }
        }
    }

    /// The corner offsets of a surface cell's empty corners (× −4) plus the
    /// pull of every neighbour that is not full (8 − its count), in quarter
    /// cells: where the corner sphere sits relative to the cell's centre.
    fn corner_direction(&self, at: [i32; 3], mask: u8) -> [i32; 3] {
        let mut dir = [0i32; 3];
        for (bit, off) in CORNER_OFFSETS.iter().enumerate() {
            if mask & (1 << bit) == 0 {
                for a in 0..3 {
                    dir[a] -= off[a] * 4;
                }
            }
        }
        for off in NEIGHBOURS {
            let n = self.fill_at([at[0] + off[0], at[1] + off[1], at[2] + off[2]]) as i32;
            if n < 8 {
                for a in 0..3 {
                    dir[a] -= (8 - n) * off[a];
                }
            }
        }
        dir
    }

    /// The first fill byte of a cell, or 0 outside the grid.
    fn fill_at(&self, cell: [i32; 3]) -> u8 {
        self.index(cell)
            .and_then(|i| self.fill.get(i))
            .map(|f| f[0])
            .unwrap_or(0)
    }
}

/// The neighbour's corners, shifted into this cell's mask and cut to the
/// ones the two share.
fn shared_corners(mask: u8, shift: i32, filter: u32) -> u8 {
    let shifted = rotate_by(mask, shift) & filter;
    (shifted & 0xff) as u8
}

/// Shifts a mask left by `shift` bits, or right by `-shift`.
fn rotate_by(mask: u8, shift: i32) -> u32 {
    let m = mask as u32;
    if shift > 0 {
        (m << shift.min(31)) & 0xffff
    } else if shift < 0 {
        m >> (-shift).min(31)
    } else {
        m
    }
}

fn popcount(mask: u8) -> u32 {
    mask.count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::mesh_touches;

    #[test]
    fn the_entries_sort_by_key() {
        let mut entries = [(3.0, 0), (1.0, 1), (2.0, 2), (1.0, 3)];
        sort_by_key(&mut entries);
        let keys: Vec<f32> = entries.iter().map(|(key, _)| *key).collect();
        assert_eq!(keys, vec![1.0, 1.0, 2.0, 3.0]);
        assert!(entries.iter().any(|(_, value)| *value == 3));
    }

    #[test]
    fn a_unit_volume_summarises_its_centre_and_mass() {
        let mut counts = vec![0; 27];
        counts[13] = 1;
        let summary = summarise([1, 1, 1], [1.0, 1.0, 1.0], &counts);
        assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
        assert!((summary.mass - 8.0).abs() < 1e-6);
    }

    #[test]
    fn an_empty_volume_has_the_fallback_mass() {
        let summary = summarise([1, 1, 1], [1.0, 1.0, 1.0], &[0; 27]);
        assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
        assert_eq!(summary.mass, EMPTY_MASS);
        // A grid with no cell along an axis is empty too.
        let summary = summarise([0, 1, 1], [1.0, 1.0, 1.0], &[0; 27]);
        assert_eq!(summary.mass, EMPTY_MASS);
    }

    #[test]
    fn the_centre_follows_the_filled_cell() {
        let mut counts = vec![0; 27];
        counts[13] = 1;
        let summary = summarise([1, 1, 1], [2.0, 4.0, 6.0], &counts);
        assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
        assert!((summary.mass - 8.0 * 48.0).abs() < 1e-5);
    }

    /// The mass multiplies in the original's order, so a volume of several
    /// cells pins the association: count · (z · (y · x)). The centre is the
    /// original's expression over the cell indices, an offset from the box's
    /// centre in its half extents.
    #[test]
    fn the_mass_associates_like_the_original() {
        // Two cells of a 2 x 2 x 2 grid, at (1, 1, 1) and (1, 1, 2).
        let mut counts = vec![0; 64];
        counts[21] = 1;
        counts[22] = 1;
        let summary = summarise([2, 2, 2], [1.0, 1.0, 1.0], &counts);
        assert_eq!(summary.mass, 2.0);
        assert_eq!(summary.centre, [-0.25, -0.25, 0.0]);
    }

    /// The grid's counts: the longest axis rounds to even, is made even, is
    /// at least six cells and at most 96; the others are floored
    /// proportions, made even when asked.
    #[test]
    fn the_grid_counts_round_and_floor() {
        let options = VolumeOptions {
            resolution: 0.3,
            ..VolumeOptions::default()
        };
        let grid = Grid::new(
            Bounds {
                centre: Vec3::ZERO,
                half_extents: Vec3::new(35.0, 10.0, 10.0),
            },
            &options,
        );
        // 35·0.3 = 10.5 rounds to even 10; 10·10/35 = 2.86 floors to 2
        // (already even, so the even rule leaves it).
        assert_eq!(grid.counts, [10, 2, 2]);
        assert_eq!(grid.step, [7.0, 10.0, 10.0]);
        let grid = Grid::new(
            Bounds {
                centre: Vec3::ZERO,
                half_extents: Vec3::new(15.0, 10.0, 10.0),
            },
            &options,
        );
        // 15·0.3 = 4.5 rounds to even 4, raised to the minimum 6; the
        // others floor to 4.
        assert_eq!(grid.counts, [6, 4, 4]);
    }

    /// A 20 × 20 × 4 cm box weighs its own volume: every cell of its grid
    /// is solid, six by six by two, and the centre is the box's.
    #[test]
    fn a_box_weighs_its_volume() {
        let bounds = Bounds {
            centre: Vec3::new(5.0, 6.0, 7.0),
            half_extents: Vec3::new(10.0, 10.0, 2.0),
        };
        let mesh = Mesh::new(&box_triangles(bounds));
        let options = VolumeOptions::for_bounds(&bounds, false);
        assert_eq!(options.resolution, COARSE_RESOLUTION);
        let fitted = fit(&mesh, &options);
        assert_eq!(fitted.counts, [6, 6, 2]);
        assert!(
            (fitted.summary.mass - 1600.0).abs() < 0.01,
            "mass {}",
            fitted.summary.mass
        );
        assert!(
            fitted.summary.centre[0].abs() < 1e-4,
            "{:?}",
            fitted.summary.centre
        );
        assert!(fitted.summary.centre[1].abs() < 1e-4);
        assert!(fitted.summary.centre[2].abs() < 1e-4);
        assert!(!fitted.spheres.is_empty());
        assert!(fitted.spheres.iter().all(|s| s.radius > 0.0));
        // Every sphere lies inside the box to a cell's slack.
        for s in &fitted.spheres {
            let p = s.centre - bounds.centre;
            assert!(p.x.abs() <= 10.0 + 6.0, "sphere at {p}");
            assert!(p.y.abs() <= 10.0 + 6.0);
            assert!(p.z.abs() <= 2.0 + 6.0);
        }
        // The volume weighs a mobility: 10 000 over it.
        assert!((fitted.summary.mobility() - 6.25).abs() < 1e-3);
    }

    /// A solid 20 cm cube fits 6 × 6 × 6 cells, one candidate at the middle
    /// of the inner layer: radius 3/(1 + 0.03) of a 20/6 cm cell, 9.7087 cm.
    #[test]
    fn a_cube_fits_a_head_sphere_and_its_corners() {
        let bounds = Bounds {
            centre: Vec3::ZERO,
            half_extents: Vec3::splat(10.0),
        };
        let mesh = Mesh::new(&box_triangles(bounds));
        let options = VolumeOptions::for_bounds(&bounds, false);
        let fitted = fit(&mesh, &options);
        assert_eq!(fitted.counts, [6, 6, 6]);
        assert!((fitted.summary.mass - 8000.0).abs() < 0.01);
        assert_eq!(fitted.summary.centre, [0.0, 0.0, 0.0]);
        let head = fitted.spheres[0];
        assert!(head.centre.length() < 1e-5, "{:?}", head.centre);
        let cell = 20.0 / 6.0;
        let radius = (3.0f64 / (3.0 * 0.01 + 1.0) * f64::from(cell)) as f32;
        assert!((head.radius - radius).abs() < 1e-4, "{}", head.radius);
        // The rest are the corner spheres of the outer layer, one cell
        // across, on the box's faces.
        assert!(fitted.spheres.len() > 1);
        assert!(
            fitted
                .spheres
                .iter()
                .skip(1)
                .all(|s| (s.radius - cell).abs() < 1e-4)
        );
    }

    /// The four point masses are 4 over the barycentric weights of the
    /// centre on the rest tetrahedron.
    #[test]
    fn the_volume_weighs_the_point_masses() {
        let rest = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
        let summary = VolumeSummary {
            centre: [0.25, 0.25, 0.25],
            mass: 1.0,
        };
        assert_eq!(summary.point_masses(rest), [16.0; 4]);
        // Weights ½, ¼, ⅛, ⅛: masses 8, 16, 32, 32.
        let summary = VolumeSummary {
            centre: [0.25, 0.125, 0.125],
            mass: 1.0,
        };
        assert_eq!(summary.point_masses(rest), [8.0, 16.0, 32.0, 32.0]);
        // An empty volume and a flat tetrahedron fall back to equal masses.
        assert_eq!(
            VolumeSummary {
                centre: [0.0; 3],
                mass: EMPTY_MASS
            }
            .point_masses(rest),
            [4.0; 4]
        );
        assert_eq!(
            summary.point_masses([Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 0.0)]),
            [4.0; 4]
        );
    }

    /// The corner mask rotation: a single corner bit turns into the next
    /// corner about the x axis, as the original's three tables do.
    #[test]
    fn the_corner_masks_rotate() {
        assert_eq!(rotate_mask(0b0000_0001), 0b1000_0000);
        // The third table does not wrap bit 7 back: 0b10 goes 4, 6, then
        // 6 again (the original's masks are 16-bit, so the top bit is lost).
        assert_eq!(rotate_mask(0b0000_0010), 0b0100_0000);
        assert_eq!(rotate_mask(0b1111_1111), 0b1111_1111);
        // Shifts: a face neighbour in -x shifts a mask left four.
        assert_eq!(corner_shift(-1, 0, 0).0, 4);
        assert_eq!(corner_shift(1, 0, 0).0, -4);
        assert_eq!(corner_shift(0, 1, 0).0, -2);
        assert_eq!(corner_shift(0, 0, 1).0, -1);
        assert_eq!(rotate_by(1, 4), 16);
        assert_eq!(rotate_by(16, -4), 1);
    }

    /// A degenerate (flat) mesh still fits, with the fallback's spheres and
    /// no panic.
    #[test]
    fn a_flat_mesh_still_fits() {
        let bounds = Bounds {
            centre: Vec3::ZERO,
            half_extents: Vec3::new(10.0, 10.0, 3.0),
        };
        let mesh = Mesh::new(&box_triangles(bounds));
        let options = VolumeOptions::for_bounds(&bounds, false);
        let fitted = fit(&mesh, &options);
        assert!(!fitted.spheres.is_empty());
        assert!(fitted.summary.mass > 0.0);
    }

    /// A ray meets a box's walls: from the centre toward +x it reaches the
    /// wall at 10 cm, and a ray pointing away misses.
    #[test]
    fn rays_meet_the_walls() {
        let triangles = box_triangles(Bounds {
            centre: Vec3::ZERO,
            half_extents: Vec3::splat(10.0),
        });
        assert!(ray_touches(&triangles, Vec3::ZERO, Vec3::X, 10.2));
        assert!(!ray_touches(&triangles, Vec3::ZERO, Vec3::X, 9.0));
        assert!(!ray_touches(
            &triangles,
            Vec3::new(20.0, 0.0, 0.0),
            Vec3::X,
            100.0
        ));
    }

    /// The bounding-volume tree answers exactly as a test of every
    /// triangle does, for sphere and ray queries over a mesh of many
    /// triangles.
    #[test]
    fn the_tree_matches_testing_every_triangle() {
        let bounds = Bounds {
            centre: Vec3::new(1.0, -2.0, 3.0),
            half_extents: Vec3::new(10.0, 6.0, 4.0),
        };
        let triangles = subdivided_box(bounds, 4);
        assert!(triangles.len() > 150);
        let mesh = Mesh::new(&triangles);
        assert_eq!(mesh.bounds().centre, bounds.centre);
        assert_eq!(mesh.bounds().half_extents, bounds.half_extents);
        let mut tested = 0;
        for i in -4..=4 {
            for j in -4..=4 {
                for k in -4..=4 {
                    let p = bounds.centre + Vec3::new(i as f32, j as f32, k as f32) * 3.0;
                    for r in [0.5f32, 2.0, 5.0] {
                        assert_eq!(
                            mesh.touches(p, r),
                            mesh_touches(&triangles, p, r),
                            "touch at {p} r {r}"
                        );
                        tested += 1;
                    }
                    for dir in [Vec3::X, -Vec3::Y, Vec3::new(0.3, 0.5, 0.8).normalize()] {
                        assert_eq!(
                            mesh.ray(p, dir, 40.0),
                            ray_touches(&triangles, p, dir, 40.0),
                            "ray at {p} along {dir}"
                        );
                        tested += 1;
                    }
                }
            }
        }
        assert!(tested > 1000);
    }

    /// A box's sides cut into `n` by `n` quads each, outward-facing.
    fn subdivided_box(b: Bounds, n: usize) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        let at = |axis: usize, sign: f32, u: f32, v: f32| {
            let mut p = b.centre;
            let (a, c) = match axis {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            p[axis] += sign * b.half_extents[axis];
            p[a] += (u - 0.5) * 2.0 * b.half_extents[a];
            p[c] += (v - 0.5) * 2.0 * b.half_extents[c];
            p
        };
        for axis in 0..3 {
            for sign in [-1.0f32, 1.0] {
                for i in 0..n {
                    for j in 0..n {
                        let (u0, u1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
                        let (v0, v1) = (j as f32 / n as f32, (j + 1) as f32 / n as f32);
                        let a = at(axis, sign, u0, v0);
                        let b2 = at(axis, sign, u1, v0);
                        let c = at(axis, sign, u1, v1);
                        let d = at(axis, sign, u0, v1);
                        out.push([a, b2, c]);
                        out.push([a, c, d]);
                    }
                }
            }
        }
        out
    }

    /// The 12 triangles of an axis-aligned box's sides.
    pub(super) fn box_triangles(b: Bounds) -> Vec<[Vec3; 3]> {
        let min = b.centre - b.half_extents;
        let max = b.centre + b.half_extents;
        let v = |x: f32, y: f32, z: f32| {
            Vec3::new(
                if x < 0.5 { min.x } else { max.x },
                if y < 0.5 { min.y } else { max.y },
                if z < 0.5 { min.z } else { max.z },
            )
        };
        let mut out = Vec::new();
        let faces = [
            [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0],
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
        ];
        for f in faces {
            out.push([
                v(f[0], f[1], f[2]),
                v(f[3], f[4], f[5]),
                v(f[6], f[7], f[8]),
            ]);
            out.push([
                v(f[0], f[1], f[2]),
                v(f[6], f[7], f[8]),
                v(f[9], f[10], f[11]),
            ]);
        }
        out
    }
}
