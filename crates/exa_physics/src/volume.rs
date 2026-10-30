//! The dynamics volumes (the original's `RF_Dynamics_Volumes`): the
//! rigid-body shape builders' voxel fit of a mesh, over the voxeliser's
//! [`VoxelMap`].
//!
//! A body built from a mesh is fitted in the original's order ([`fit`]):
//! the grid over the mesh's box and its near-surface fill
//! ([`fill_near_surface`]), the six-ray solid fill ([`fill_solid`]), the
//! volume and its centre ([`summarise`]), the corner masks
//! ([`mark_corners`]) and the sphere extraction ([`extract_spheres`], which
//! falls back to [`fallback_spheres`] for a mesh too thin for any inner
//! sphere). The queries run against the mesh's triangles as the original's
//! mesh kd-tree tests them ([`Mesh`]).
//!
//! The volume weighs the body: its mobility is 10 000 over the volume, and
//! its four point masses are 4 over the barycentric weights of the centre
//! on its rest tetrahedron ([`VolumeSummary::mobility`],
//! [`VolumeSummary::point_masses`]). There is no buoyancy or drag in the
//! original's physics, so none is modelled here.
//!
//! Lengths are the original's centimetres, and the spheres come out in the
//! mesh's own space. The triangle queries take the processor's approximate
//! reciprocal and reciprocal square root where the original does, so a
//! cell exactly at a query's radius falls the way it does in the original
//! on the same processor.

use bevy::math::Vec3;

use crate::voxel::{Bounds, Sphere, VoxelMap};

/// A volume's centre and mass, as the original's summary returns them.
///
/// The centre is the original's expression over the filled cells' indices,
/// `(mean index − ½)/count − ½` times the half extent on each axis: an
/// offset from the map's centre that is half the offset of the filled
/// cells' centroid (the original's own arithmetic, which its builders use
/// as it is).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeSummary {
    pub centre: [f32; 3],
    pub mass: f32,
}

/// The mass returned for a map with an axis of no cells (0.1).
pub const EMPTY_MASS: f32 = 0.1;

/// The volume a body of mobility 1 has: mobility = 10 000 / volume.
pub const MOBILITY_VOLUME: f32 = 10_000.0;

/// The point-mass numerator: each mass is 4 over its weight.
pub const POINT_MASS: f32 = 4.0;

/// The resolution the builders ask for, per cm of the mesh's longest half
/// extent, when the smallest half extent is under 10 cm.
pub const COARSE_RESOLUTION: f32 = 0.6;
/// The resolution when the smallest half extent is at least 10 cm.
pub const FINE_RESOLUTION: f32 = 0.3;
/// The smallest half extent that switches to the fine resolution.
pub const FINE_HALF_EXTENT: f32 = 10.0;
/// The resolution with the builders' high-quality flag: one cell per cm.
pub const PRECISE_RESOLUTION: f32 = 1.0;
/// The fewest cells along the longest axis.
pub const MIN_DEPTH: i32 = 6;
/// The fewest cells along the longest axis with the high-quality flag.
pub const PRECISE_MIN_DEPTH: i32 = 8;
/// The most cells along the longest axis.
pub const MAX_DEPTH: i32 = 96;
/// The margin the mesh tree's box takes on each side of the mesh's box.
pub const TREE_MARGIN: f32 = 0.01;
/// How far a ray along +x, +y or +z reaches past the tree's box (a double
/// in the original); the rays along −x, −y and −z stop at it.
pub const RAY_OVERREACH: f64 = 0.2;

impl VolumeSummary {
    /// The mobility a body of this volume has (10 000 / volume).
    pub fn mobility(&self) -> f32 {
        MOBILITY_VOLUME / self.mass
    }

    /// The four point masses of a rigid body with rest tetrahedron `rest`:
    /// 4 over each vertex's barycentric weight of the point `centre +
    /// origin`, where the builders pass the mesh's box centre as `origin`.
    ///
    /// The original stores whatever the division gives; a flat tetrahedron
    /// or a centre on a face would give zero, infinite or NaN masses, so
    /// those come back as four equal masses here (Exarust's guard).
    pub fn point_masses(&self, origin: Vec3, rest: [Vec3; 4]) -> [f32; 4] {
        let c = self.centre;
        let point = Vec3::new(c[0] + origin.x, c[1] + origin.y, c[2] + origin.z);
        let w = barycentric_weights(point, rest);
        if w.iter().all(|w| w.is_finite() && *w != 0.0) {
            w.map(|w| POINT_MASS / w)
        } else {
            [POINT_MASS; 4]
        }
    }
}

/// `a · (b × c)`, the cross product's terms and the dot's sum in the
/// original's order.
fn triple(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let cross = Vec3::new(
        c.z * b.y - b.z * c.y,
        c.x * b.z - b.x * c.z,
        c.y * b.x - b.y * c.x,
    );
    dot(a, cross)
}

/// A dot product summed as the original's vector code does: `(x + y) + z`.
fn dot(a: Vec3, b: Vec3) -> f32 {
    (a.x * b.x + a.y * b.y) + a.z * b.z
}

/// The barycentric weights of `point` on the tetrahedron `rest`, as the
/// original's helper works them out: four triple products over the
/// tetrahedron's, multiplied by one over it.
pub fn barycentric_weights(point: Vec3, rest: [Vec3; 4]) -> [f32; 4] {
    let [v0, v1, v2, v3] = rest;
    let w0 = triple(point - v1, v3 - v1, v2 - v1);
    let (p, e1, e2, e3) = (point - v0, v1 - v0, v2 - v0, v3 - v0);
    let w1 = triple(p, e2, e3);
    let w2 = triple(p, e3, e1);
    let w3 = triple(p, e1, e2);
    let inverse = 1.0 / triple(e1, e2, e3);
    [inverse * w0, inverse * w1, inverse * w2, inverse * w3]
}

/// Sorts entries by their float key with the original's quicksort: a middle
/// pivot, the left scan stopping at a value at or above it and the right one
/// at a value at or below it, a swap while the two have not crossed; the
/// left side then recurses while the right is walked by looping, with the
/// pivot of every pass taken halfway between the current low and the high
/// this call was given.
pub fn sort_by_key(entries: &mut [(f32, u32)]) {
    if entries.len() > 1 {
        quicksort(entries, 0, entries.len() as isize - 1);
    }
}

fn quicksort(entries: &mut [(f32, u32)], low: isize, high: isize) {
    let mut low = low;
    loop {
        let pivot = entries[((low + high) / 2) as usize].0;
        let mut i = low;
        let mut j = high;
        loop {
            // The original's scans can step at most to the pivot's own entry
            // (it is the one value the two halts share), so the bounds cannot
            // be passed; they are still checked so a malformed key cannot
            // panic. A NaN key halts both scans at once.
            while i < high && entries[i as usize].0 < pivot {
                i += 1;
            }
            while low < j && pivot < entries[j as usize].0 {
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
        low = i;
        if i >= high {
            break;
        }
    }
}

/// How the shape builders make a body's voxel grid and spheres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeOptions {
    /// Cells per cm of the longest half extent.
    pub resolution: f32,
    /// The fewest cells along the longest axis.
    pub min_depth: i32,
    /// Whether the other axes are made even too (the limb and root
    /// builders pass true; a factory weapon's record says).
    pub even_axes: bool,
    /// A cell needs more occupied corners than this to give a corner
    /// sphere (the builders pass 1).
    pub min_corners: i32,
    /// A corner sphere needs more corners left unexplained than this (the
    /// limb and root builders pass 0; a factory weapon's record says).
    pub min_free_corners: i32,
    /// The voxeliser's global flag for the wider queries: the near-surface
    /// sphere of `√2/2` of half a cell and the inner one of `0.75·√2`
    /// (which setting raises it is not established; off by default).
    pub wide_queries: bool,
}

impl Default for VolumeOptions {
    fn default() -> Self {
        Self {
            resolution: FINE_RESOLUTION,
            min_depth: MIN_DEPTH,
            even_axes: true,
            min_corners: 1,
            min_free_corners: 0,
            wide_queries: false,
        }
    }
}

impl VolumeOptions {
    /// The options the limb and root builders choose for a mesh with these
    /// bounds: the fine resolution when its smallest half extent is at
    /// least 10 cm (or is NaN), else the coarse one; with `precise` (the
    /// builders' high-quality flag) one cell per cm and eight cells at
    /// least.
    pub fn for_bounds(bounds: &Bounds, precise: bool) -> Self {
        if precise {
            return Self {
                resolution: PRECISE_RESOLUTION,
                min_depth: PRECISE_MIN_DEPTH,
                ..Self::default()
            };
        }
        // The smallest half extent, x first, a later axis taken when it is
        // smaller (NaN never is).
        let h = bounds.half_extents;
        let mut smallest = h.x;
        for v in [h.y, h.z] {
            if v < smallest {
                smallest = v;
            }
        }
        Self {
            resolution: if FINE_HALF_EXTENT > smallest {
                COARSE_RESOLUTION
            } else {
                FINE_RESOLUTION
            },
            ..Self::default()
        }
    }
}

/// The processor's approximate reciprocal (RCPSS), as the original's mesh
/// queries take it; exact off x86-64.
fn approximate_reciprocal(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // The same approximate reciprocal the original's triangle code uses.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value
    }
}

/// The squared distance a sphere query must beat (one ulp-sized margin
/// under the squared radius, the original's epsilon).
const TOUCH_EPSILON: f32 = 1.1920899822825959e-07;
/// The smallest ray–face cosine and hit distance the ray query accepts.
const RAY_EPSILON: f32 = 1e-8;

/// One triangle as the original's mesh keeps it for its queries: the unit
/// normal (scaled by the approximate reciprocal square root, zero for a
/// degenerate face), the plane's constant, the first edge's squared length
/// and the approximate reciprocal of the Gram determinant of the two edges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    pub vertices: [Vec3; 3],
    pub normal: Vec3,
    pub plane: f32,
    pub edge_squared: f32,
    pub inverse_gram: f32,
}

impl Face {
    /// Sets a triangle up as the original's record builder does.
    pub fn new(vertices: [Vec3; 3]) -> Self {
        let [v0, v1, v2] = vertices;
        let u = v1 - v0;
        let w = v2 - v0;
        let cross = Vec3::new(
            w.z * u.y - u.z * w.y,
            w.x * u.z - u.x * w.z,
            w.y * u.x - u.y * w.x,
        );
        let length_squared = dot(cross, cross);
        let scale = if length_squared == 0.0 {
            0.0
        } else {
            crate::engine_math::reciprocal_sqrt(length_squared)
        };
        let normal = cross * scale;
        let edge_squared = dot(u, u);
        let uw = dot(u, w);
        let gram = edge_squared * dot(w, w) - uw * uw;
        Self {
            vertices,
            normal,
            plane: dot(v0, normal),
            edge_squared,
            inverse_gram: approximate_reciprocal(gram),
        }
    }

    /// Whether the sphere touches the face as the original's sphere query
    /// tests it: its centre no farther from the plane than the radius (NaN
    /// passes), and its squared distance to the face's nearest point (as
    /// [`Face::nearest`] finds it) under the squared radius less the
    /// epsilon.
    pub fn touches(&self, centre: Vec3, radius: f32) -> bool {
        let distance = dot(centre, self.normal) - self.plane;
        if distance.abs() > radius {
            return false;
        }
        let d = self.nearest(centre, distance) - centre;
        let squared = dot(d, d);
        radius * radius - TOUCH_EPSILON > squared
    }

    /// The original's nearest point of the face to `p`, given `p`'s
    /// distance to the plane: `p` projected on the plane, then the region
    /// test on the two edges from the first vertex (the third edge only
    /// when a projection runs past the second vertex or the face), each
    /// edge parameter taken with the approximate reciprocal.
    pub fn nearest(&self, p: Vec3, distance: f32) -> Vec3 {
        let [v0, v1, v2] = self.vertices;
        let q = (p - self.normal * distance) - v0;
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let a = self.edge_squared;
        let b = dot(e1, e2);
        let c1 = dot(e1, q);
        let c2 = dot(e2, q);
        let s = a * c2 - b * c1;
        if s > 0.0 {
            let d2 = dot(e2, e2);
            let t = c1 * d2 - b * c2;
            if t > 0.0 {
                if (s + t) * self.inverse_gram > 1.0 {
                    return self.nearest_on_far_edge(q, e1);
                }
                return q + v0;
            }
            if c2 > d2 {
                return self.nearest_on_far_edge(q, e1);
            }
            let k = maximum(approximate_reciprocal(d2) * c2, 0.0);
            return v0 + e2 * k;
        }
        if c1 > 0.0 {
            if c1 > a {
                return self.nearest_on_far_edge(q, e1);
            }
            let k = approximate_reciprocal(a) * c1;
            return v0 + e1 * k;
        }
        let d2 = dot(e2, e2);
        let k = minimum(maximum(approximate_reciprocal(d2) * c2, 0.0), 1.0);
        e2 * k + v0
    }

    /// The nearest point on the edge from the second vertex to the third,
    /// its parameter clamped to [0, 1].
    fn nearest_on_far_edge(&self, q: Vec3, e1: Vec3) -> Vec3 {
        let [_, v1, v2] = self.vertices;
        let f = v2 - v1;
        let g = q - e1;
        let k = minimum(
            maximum(approximate_reciprocal(dot(f, f)) * dot(g, f), 0.0),
            1.0,
        );
        f * k + v1
    }

    /// Whether the ray from `origin` along `direction` meets the face
    /// within `length`, as the original's ray query tests it: only faces
    /// whose normal points along the ray (cosine at least 1e-8), a hit
    /// distance from 1e-8 to the length, and a barycentric test on the two
    /// axes other than the normal's largest (NaN passes each test).
    pub fn ray_hits(&self, origin: Vec3, direction: Vec3, length: f32) -> bool {
        let cosine = dot(direction, self.normal);
        if cosine < RAY_EPSILON {
            return false;
        }
        let along = -(dot(origin, self.normal) - self.plane) / cosine;
        if along < RAY_EPSILON || along > length {
            return false;
        }
        let n = self.normal.abs();
        // The first test axis: 1 when x dominates, 2 when y does, else 0;
        // the second follows it (1, 2, 0).
        let first = if n.y < n.x {
            if n.z < n.x { 1 } else { 0 }
        } else if n.z < n.y {
            2
        } else {
            0
        };
        let second = [1, 2, 0][first];
        let [v0, v1, v2] = self.vertices.map(|v| v.to_array());
        let e1a = v1[first] - v0[first];
        let e1b = v1[second] - v0[second];
        let e2a = v2[first] - v0[first];
        let e2b = v2[second] - v0[second];
        let inverse = 1.0 / (e2a * e1b - e2b * e1a);
        let (o, d) = (origin.to_array(), direction.to_array());
        let pa = ((d[first] * along + o[first]) - v0[first]) * inverse;
        let pb = ((d[second] * along + o[second]) - v0[second]) * inverse;
        let u = pb * e2a - pa * e2b;
        if u < 0.0 {
            return false;
        }
        let v = pa * e1b - pb * e1a;
        if v < 0.0 {
            return false;
        }
        // NaN passes, as in the original.
        (u + v).partial_cmp(&1.0) != Some(std::cmp::Ordering::Greater)
    }
}

/// The processor's MAXSS: `a` when it is the larger, else `b` (so `b` on
/// NaN).
fn maximum(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// The processor's MINSS: `a` when it is the smaller, else `b` (so `b` on
/// NaN).
fn minimum(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// A mesh to fit a volume to: its faces as the original's queries test
/// them, the box the voxel grid is made over, and a bounding-volume tree so
/// the queries do not test every face of a large mesh (the original's mesh
/// kd-tree; the tree only skips faces that cannot answer, so every query
/// answers as a test of every face does).
#[derive(Debug, Clone)]
pub struct Mesh {
    faces: Vec<Face>,
    bounds: Bounds,
    /// The tree's box: the mesh's box widened by [`TREE_MARGIN`].
    low: Vec3,
    high: Vec3,
    /// Nodes depth first: a leaf holds `count` faces from `first`, an inner
    /// node two children at `left` and `right`.
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
    /// A mesh over its triangles, its box the union of their boxes.
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
        Self::with_bounds(triangles, bounds)
    }

    /// A mesh over its triangles with the box it carries (the original
    /// makes the grid over the mesh's own stored box, centre and half
    /// extents).
    pub fn with_bounds(triangles: &[[Vec3; 3]], bounds: Bounds) -> Self {
        let faces: Vec<Face> = triangles.iter().map(|&t| Face::new(t)).collect();
        let mut order: Vec<u32> = (0..faces.len() as u32).collect();
        let mut nodes = Vec::new();
        if !faces.is_empty() {
            let len = order.len();
            build_node(&faces, &mut order, 0, len, &mut nodes);
        }
        let margin = Vec3::splat(TREE_MARGIN);
        Self {
            low: (bounds.centre - bounds.half_extents) - margin,
            high: (bounds.centre + bounds.half_extents) + margin,
            faces,
            bounds,
            nodes,
            order,
        }
    }

    /// The mesh's box: the grid's centre and half extents.
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// The tree's box, the mesh's box widened by [`TREE_MARGIN`]: the rays
    /// run to its faces.
    pub fn tree_box(&self) -> (Vec3, Vec3) {
        (self.low, self.high)
    }

    pub fn faces(&self) -> &[Face] {
        &self.faces
    }

    /// Whether the sphere touches any face ([`Face::touches`]).
    pub fn touches(&self, centre: Vec3, radius: f32) -> bool {
        // A face can only touch when the sphere reaches its node's box,
        // which is widened past any rounding of the nearest point.
        let reach = radius.abs() * 1.001 + 1e-3;
        let limit = reach * reach;
        self.walk(
            |lo, hi| {
                box_distance_squared(centre, lo, hi).partial_cmp(&limit)
                    != Some(std::cmp::Ordering::Greater)
            },
            |face| face.touches(centre, radius),
        )
    }

    /// Whether the ray from `origin` along `direction` (unit) hits any face
    /// within `length` ([`Face::ray_hits`]).
    pub fn ray(&self, origin: Vec3, direction: Vec3, length: f32) -> bool {
        self.walk(
            |lo, hi| segment_meets_box(origin, direction, length, lo, hi),
            |face| face.ray_hits(origin, direction, length),
        )
    }

    fn walk(&self, enter: impl Fn(Vec3, Vec3) -> bool, test: impl Fn(&Face) -> bool) -> bool {
        let mut stack = vec![0u32];
        while let Some(index) = stack.pop() {
            let Some(node) = self.nodes.get(index as usize) else {
                continue;
            };
            if !enter(node.lo, node.hi) {
                continue;
            }
            if node.count > 0 {
                let first = node.first as usize;
                let faces = self
                    .order
                    .get(first..first + node.count as usize)
                    .unwrap_or(&[]);
                if faces
                    .iter()
                    .filter_map(|&i| self.faces.get(i as usize))
                    .any(&test)
                {
                    return true;
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
/// widest axis at the median of the faces' centres, with at most eight per
/// leaf. Node boxes are widened by a thousandth of their size and a
/// thousandth of a cm, far past the rounding of a query's nearest point or
/// hit point, so the tree never skips a face that answers.
fn build_node(
    faces: &[Face],
    order: &mut [u32],
    start: usize,
    count: usize,
    nodes: &mut Vec<Node>,
) -> u32 {
    const LEAF: usize = 8;
    let centre_of = |i: u32| {
        let [a, b, c] = faces[i as usize].vertices;
        (a + b + c) / 3.0
    };
    let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
    let (mut clo, mut chi) = (Vec3::MAX, Vec3::MIN);
    for &i in &order[start..start + count] {
        let [a, b, c] = faces[i as usize].vertices;
        let m = centre_of(i);
        clo = clo.min(m);
        chi = chi.max(m);
        lo = lo.min(a.min(b).min(c));
        hi = hi.max(a.max(b).max(c));
    }
    let margin =
        Vec3::splat((hi - lo).max_element().max(0.0) * 1e-3 + 1e-3) + lo.abs().max(hi.abs()) * 1e-5;
    let index = nodes.len() as u32;
    nodes.push(Node {
        lo: lo - margin,
        hi: hi + margin,
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
    order[start..start + count].sort_by(|&a, &b| centre_of(a)[axis].total_cmp(&centre_of(b)[axis]));
    let middle = count / 2;
    let left = build_node(faces, order, start, middle, nodes);
    let right = build_node(faces, order, start + middle, count - middle, nodes);
    if let Some(node) = nodes.get_mut(index as usize) {
        node.count = 0;
        node.left = left;
        node.right = right;
    }
    index
}

/// Squared distance from a point to an axis-aligned box (0 inside).
fn box_distance_squared(p: Vec3, lo: Vec3, hi: Vec3) -> f32 {
    let d = (lo - p).max(Vec3::ZERO).max(p - hi);
    d.length_squared()
}

/// Whether the segment `origin .. origin + direction · length` meets the
/// box (a NaN anywhere keeps the box).
fn segment_meets_box(origin: Vec3, direction: Vec3, length: f32, lo: Vec3, hi: Vec3) -> bool {
    let mut near = 0.0f32;
    let mut far = if length.is_nan() {
        f32::INFINITY
    } else {
        length
    };
    for a in 0..3 {
        let d = direction[a];
        if d == 0.0 {
            if origin[a] < lo[a] || origin[a] > hi[a] {
                return false;
            }
            continue;
        }
        let (mut t0, mut t1) = ((lo[a] - origin[a]) / d, (hi[a] - origin[a]) / d);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > near {
            near = t0;
        }
        if t1 < far {
            far = t1;
        }
        if near > far {
            return false;
        }
    }
    true
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

/// The tag the limb and root builders make their map with (the mesh tree's
/// per-thread stack slot their queries use).
pub const BUILDER_MAP_TAG: u32 = 2;

/// The shape builders' voxel fit, in their order: the grid and its
/// near-surface fill, the solid fill, the volume and centre, the corner
/// masks and the sphere extraction. The mesh's triangles and box must be in
/// the original's centimetres.
pub fn fit(mesh: &Mesh, options: &VolumeOptions) -> FittedVolume {
    let mut map = VoxelMap::new(BUILDER_MAP_TAG);
    fill_near_surface(&mut map, mesh, options);
    fill_solid(&mut map, mesh);
    let summary = summarise(&map);
    mark_corners(&mut map);
    extract_spheres(&mut map, options);
    FittedVolume {
        counts: map.counts.map(|c| c as i32),
        summary,
        spheres: map.spheres().to_vec(),
    }
}

/// The 26 neighbours, in the original's table order: x, then y, then z.
///
/// Every scan reads all 26: the original's loops count up from 0 and test
/// the index against 25 after the body, so the last entry, `(+1, +1, +1)`,
/// is walked too.
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

/// A cell's eight corner cells, as offsets: bit `4x + 2y + z` of a corner
/// mask is the cell at `(x, y, z)` from it (the original's mask order).
const CORNER_CELLS: [[i32; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 0],
    [0, 1, 1],
    [1, 0, 0],
    [1, 0, 1],
    [1, 1, 0],
    [1, 1, 1],
];

/// The direction of each corner from the corner point, in half cells: the
/// original's corner table for the corner spheres' offsets.
const CORNER_DIRECTIONS: [[i32; 3]; 8] = [
    [-1, -1, -1],
    [-1, -1, 1],
    [-1, 1, -1],
    [-1, 1, 1],
    [1, -1, -1],
    [1, -1, 1],
    [1, 1, -1],
    [1, 1, 1],
];

/// The six face neighbours, in the original's order: −x, +x, −y, +y, −z, +z.
const FACE_NEIGHBOURS: [[i32; 3]; 6] = [
    [-1, 0, 0],
    [1, 0, 0],
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
];

/// The original's three neighbour tables, one per axis, indexed by the
/// neighbour's offset −1, 0, +1 on it: how far the neighbour's corner mask
/// shifts to land on this cell's corners (a corner bit is `4x + 2y + z`)
/// and which of this cell's corners the two share.
const SHARE_X: [(i32, u8); 3] = [(-4, 0x0f), (0, 0xff), (4, 0xf0)];
const SHARE_Y: [(i32, u8); 3] = [(-2, 0x33), (0, 0xff), (2, 0xcc)];
const SHARE_Z: [(i32, u8); 3] = [(-1, 0x55), (0, 0xff), (1, 0xaa)];

/// The corners of this cell that the neighbour at `offset` (with corner
/// mask `mask`) also has.
fn shared_corners(mask: u8, offset: [i32; 3]) -> u8 {
    let [x, y, z] = offset.map(|o| (o + 1) as usize);
    let shift = SHARE_X[x].0 + SHARE_Y[y].0 + SHARE_Z[z].0;
    let keep = SHARE_X[x].1 & SHARE_Y[y].1 & SHARE_Z[z].1;
    let moved = if shift < 1 {
        u32::from(mask) >> (-shift)
    } else {
        u32::from(mask) << shift
    };
    (moved & u32::from(keep)) as u8
}

/// A corner mask mirrored through the cell's centre (the original's three
/// swap tables: x halves, then y pairs, then z bits), so corner `c` becomes
/// corner `7 − c`.
fn mirror_mask(mask: u8) -> u8 {
    let m = (mask & 0x0f) << 4 | (mask & 0xf0) >> 4;
    let m = (m & 0x33) << 2 | (m & 0xcc) >> 2;
    (m & 0x55) << 1 | (m & 0xaa) >> 1
}

/// The map's counts as the original's signed words.
fn counts_of(map: &VoxelMap) -> [i32; 3] {
    map.counts.map(|c| c as i32)
}

/// The linear strides of the bordered grid: a step in x, in y (z is 1).
fn strides(counts: [i32; 3]) -> (i32, i32) {
    let nz = counts[2] + 2;
    ((counts[1] + 2) * nz, nz)
}

/// `n · a / b` rounded towards +∞ as the original's round helper does (it
/// sets the processor's rounding mode up for one conversion), into a 32-bit
/// word (the conversion's out-of-range answer is `i32::MIN`).
fn ceil_ratio(n: i32, a: f32, b: f32) -> i32 {
    let v = (n as f32 * a / b).ceil();
    if v.is_nan() || v >= 2_147_483_648.0 || v < -2_147_483_648.0 {
        i32::MIN
    } else {
        v as i32
    }
}

/// The processor's rounding conversion (CVTSS2SI, nearest, ties to even)
/// into a 64-bit word, kept as its low 32 bits.
fn round_to_word(v: f32) -> i32 {
    let r = v.round_ties_even();
    if r.is_nan() || r >= 9.223_372e18 || r < -9.223_372e18 {
        0
    } else {
        r as i64 as i32
    }
}

/// The original's grid and near-surface fill. The map's box is the mesh's;
/// the longest axis (x first, a later axis taken only when strictly longer)
/// gets `round(half · resolution)` cells, made even, at least the minimum
/// and at most 96, and the other axes `n · half / longest half` rounded up,
/// made even when asked. A longest half extent under 1 cm makes no grid.
///
/// Every cell whose centre is within half a longest-axis cell of the
/// surface is marked 1 ([`Mesh::touches`]); then every empty cell with at
/// least five marked neighbours whose offsets sum to at most 3 on each
/// axis, and whose centre touches the surface again, is marked 2; and
/// those become 1. Returns whether a grid was made.
pub fn fill_near_surface(map: &mut VoxelMap, mesh: &Mesh, options: &VolumeOptions) -> bool {
    let bounds = mesh.bounds();
    let half = bounds.half_extents;
    let mut axis = 0;
    let mut longest = half.x;
    for i in 1..3 {
        if longest < half[i] {
            longest = half[i];
            axis = i;
        }
    }
    if longest < 1.0 {
        map.centre = bounds.centre;
        map.half_extents = half;
        return false;
    }
    let mut counts = [0i32; 3];
    let mut n = round_to_word(longest * options.resolution);
    if n & 1 != 0 {
        n = n.wrapping_add(1);
    }
    if options.min_depth > n {
        n = options.min_depth;
    }
    if n > MAX_DEPTH {
        n = MAX_DEPTH;
    }
    counts[axis] = n;
    for (i, count) in counts.iter_mut().enumerate() {
        if i == axis {
            continue;
        }
        let mut m = ceil_ratio(n, half[i], longest);
        if options.even_axes && m & 1 != 0 {
            m = m.wrapping_add(1);
        }
        *count = m;
    }
    // The original clamps a negative count to one only after its first
    // pass (a NaN extent's count); here before the grid is made, so the
    // grid stays in bounds.
    for c in &mut counts {
        if *c < 0 {
            *c = 1;
        }
    }
    if !map.resize(bounds.centre, half, counts.map(|c| c as usize)) {
        return false;
    }
    let (sx, sy) = strides(counts);
    let step = map.step();
    let low = |a: usize| {
        let h = half[a];
        (bounds.centre[a] - h) - h / counts[a] as f32
    };
    let low = [low(0), low(1), low(2)];
    let centre_of = |i: i32, j: i32, k: i32| {
        Vec3::new(
            i as f32 * step.x + low[0],
            j as f32 * step.y + low[1],
            k as f32 * step.z + low[2],
        )
    };
    let half_cell = half[axis] / counts[axis] as f32;
    let near = if options.wide_queries {
        (f64::from(half_cell) * std::f64::consts::SQRT_2 * 0.5) as f32
    } else {
        half_cell * 1.0
    };
    let [nx, ny, nz] = counts;
    let cells = map.cells_mut();
    for i in 1..=nx {
        for j in 1..=ny {
            for k in 1..=nz {
                if mesh.touches(centre_of(i, j, k), near)
                    && let Some(c) = cells.get_mut((sx * i + sy * j + k) as usize)
                {
                    *c = 1;
                }
            }
        }
    }
    let inner = if options.wide_queries {
        (f64::from(half_cell) * std::f64::consts::SQRT_2 * 0.75) as f32
    } else {
        half_cell * 1.0
    };
    let offsets = NEIGHBOURS.map(|[x, y, z]| sx * x + sy * y + z);
    for i in 1..=nx {
        for j in 1..=ny {
            for k in 1..=nz {
                let at = sx * i + sy * j + k;
                if cells.get(at as usize) != Some(&0) {
                    continue;
                }
                let (mut marked, mut sum) = (0, [0i32; 3]);
                for (offset, d) in offsets.iter().zip(NEIGHBOURS) {
                    if cells.get((at + offset) as usize) == Some(&1) {
                        marked += 1;
                        sum = [sum[0] + d[0], sum[1] + d[1], sum[2] + d[2]];
                    }
                }
                if marked >= 5
                    && sum.iter().all(|s| s.unsigned_abs() <= 3)
                    && mesh.touches(centre_of(i, j, k), inner)
                    && let Some(c) = cells.get_mut(at as usize)
                {
                    *c = 2;
                }
            }
        }
    }
    for c in cells.iter_mut() {
        if *c == 2 {
            *c = 1;
        }
    }
    true
}

/// The original's solid fill: an empty cell becomes solid when both rays
/// of every axis from its centre hit the mesh ([`Mesh::ray`]): the ray
/// toward −axis reaches the tree's box, the one toward +axis 0.2 cm past
/// it, and the second ray of an axis is cast only when the first hits.
/// Returns how many cells are filled after it.
pub fn fill_solid(map: &mut VoxelMap, mesh: &Mesh) -> i32 {
    let counts = counts_of(map);
    if map.cells().is_empty() || counts.iter().any(|&c| c < 1) {
        return 0;
    }
    let (sx, sy) = strides(counts);
    let step = map.step();
    let (centre, half) = (map.centre, map.half_extents);
    let low = |a: usize| (centre[a] - half[a]) - half[a] / counts[a] as f32;
    let low = [low(0), low(1), low(2)];
    let (min, max) = mesh.tree_box();
    let reach = |v: f32, a: usize| {
        let back = v - min[a];
        let ahead = (f64::from(max[a] - v) + RAY_OVERREACH) as f32;
        (back, ahead)
    };
    let [nx, ny, nz] = counts;
    let mut filled = 0;
    let cells = map.cells_mut();
    for i in 1..=nx {
        let x = i as f32 * step.x + low[0];
        let rx = reach(x, 0);
        for j in 1..=ny {
            let y = j as f32 * step.y + low[1];
            let ry = reach(y, 1);
            for k in 1..=nz {
                let at = (sx * i + sy * j + k) as usize;
                if cells.get(at) != Some(&0) {
                    filled += 1;
                    continue;
                }
                let z = k as f32 * step.z + low[2];
                let rz = reach(z, 2);
                let origin = Vec3::new(x, y, z);
                let mut axes = 0;
                for (a, (back, ahead)) in [rx, ry, rz].into_iter().enumerate() {
                    let mut direction = Vec3::ZERO;
                    direction[a] = -1.0;
                    if !mesh.ray(origin, direction, back) {
                        continue;
                    }
                    direction[a] = 1.0;
                    if mesh.ray(origin, direction, ahead) {
                        axes += 1;
                        if axes > 2 {
                            break;
                        }
                    }
                }
                if axes > 2
                    && let Some(c) = cells.get_mut(at)
                {
                    *c = 1;
                    filled += 1;
                }
            }
        }
    }
    filled
}

/// Summarises the map's filled cells (any non-zero word, border rows at the
/// low end included) in the original's arithmetic: the index sums run
/// `(i + sum) − ½`, the centre is `(sum/count/n − ½) · half · 1`, and the
/// mass `count · ((2·hz/nz) · ((2·hy/ny) · (2·hx/nx)))`. A map with an axis
/// of no cells has centre 0 and mass [`EMPTY_MASS`]; one with no filled
/// cell has mass 0 and a NaN centre (the original's sums start from
/// whatever its stack held, values that vanish against any index, so they
/// are taken as zero here).
pub fn summarise(map: &VoxelMap) -> VolumeSummary {
    let counts = counts_of(map);
    if counts.iter().any(|&c| c < 1) {
        return VolumeSummary {
            centre: [0.0; 3],
            mass: EMPTY_MASS,
        };
    }
    let (sx, sy) = strides(counts);
    let cells = map.cells();
    let mut sum = [0.0f32; 3];
    let mut filled = 0i32;
    for i in 0..=counts[0] {
        for j in 0..=counts[1] {
            for k in 0..=counts[2] {
                if cells
                    .get((sx * i + sy * j + k) as usize)
                    .is_some_and(|&c| c != 0)
                {
                    sum[0] = (i as f32 + sum[0]) - 0.5;
                    sum[1] = (j as f32 + sum[1]) - 0.5;
                    sum[2] = (k as f32 + sum[2]) - 0.5;
                    filled += 1;
                }
            }
        }
    }
    let count = filled as f32;
    let h = map.half_extents;
    let centre = [0, 1, 2].map(|a| ((sum[a] / count) / counts[a] as f32 - 0.5) * h[a] * 1.0);
    let x = h.x * 2.0 / counts[0] as f32;
    let y = h.y * 2.0 / counts[1] as f32 * x;
    let z = h.z * 2.0 / counts[2] as f32 * y;
    VolumeSummary {
        centre,
        mass: count * z,
    }
}

/// The original's corner masks: every cell from the low border to the last
/// inner cell on each axis counts which of its eight corner cells (itself
/// and its +x, +y, +z neighbours) are filled, and keeps the count and the
/// mask in its two fill bytes. A map with an axis of no cells gets none.
pub fn mark_corners(map: &mut VoxelMap) {
    let counts = counts_of(map);
    if counts.iter().any(|&c| c < 1) {
        return;
    }
    let (sx, sy) = strides(counts);
    let offsets = CORNER_CELLS.map(|[x, y, z]| sx * x + sy * y + z);
    let cells = map.cells();
    let mut fill = vec![[0u8; 2]; cells.len()];
    for i in 0..=counts[0] {
        for j in 0..=counts[1] {
            for k in 0..=counts[2] {
                let at = sx * i + sy * j + k;
                let (mut count, mut mask) = (0u8, 0u8);
                for (bit, offset) in offsets.iter().enumerate() {
                    if cells.get((at + offset) as usize).is_some_and(|&c| c != 0) {
                        count += 1;
                        mask |= 1 << bit;
                    }
                }
                if count != 0
                    && let Some(f) = fill.get_mut(at as usize)
                {
                    *f = [count, mask];
                }
            }
        }
    }
    map.set_fill(fill);
}

/// A candidate sphere: its cell (the original keeps the indices as bytes)
/// and its radius in cells.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    cell: [i32; 3],
    radius: f32,
}

/// Adds `by` to the coverage of every cell whose corner point (its +½
/// corner) is within `limit` cells of the candidate's, over the original's
/// window: from `lim − 1` cells below to `lim` above, `lim` the radius plus
/// ½ rounded to even (the original derives that window from the candidate's
/// radius before it ever clears one, so a pruned candidate's coverage is
/// taken back over its own window).
fn cover(coverage: &mut [i32], strides: (i32, i32), c: Candidate, limit: f32, by: i32) {
    let lim = round_to_word(c.radius + 0.5);
    let squared = limit * limit;
    let [cx, cy, cz] = c.cell;
    for x in cx - lim + 1..=cx + lim {
        for y in cy - lim + 1..=cy + lim {
            for z in cz - lim + 1..=cz + lim {
                let (fx, fy, fz) = (
                    (x - cx) as f32 - 0.5,
                    (y - cy) as f32 - 0.5,
                    (z - cz) as f32 - 0.5,
                );
                // The original adds y² to x², then z² to that sum.
                if fz * fz + (fy * fy + fx * fx) <= squared
                    && let Some(n) = coverage.get_mut((strides.0 * x + strides.1 * y + z) as usize)
                {
                    *n += by;
                }
            }
        }
    }
}

/// Whether every cell of the candidate's `r − ¼` disc is covered twice
/// (by it and another).
fn redundant(coverage: &[i32], strides: (i32, i32), c: Candidate) -> bool {
    let lim = round_to_word(c.radius + 0.5);
    let t = c.radius - 0.25;
    let squared = t * t;
    let [cx, cy, cz] = c.cell;
    for x in cx - lim + 1..=cx + lim {
        for y in cy - lim + 1..=cy + lim {
            for z in cz - lim + 1..=cz + lim {
                let (fx, fy, fz) = (
                    (x - cx) as f32 - 0.5,
                    (y - cy) as f32 - 0.5,
                    (z - cz) as f32 - 0.5,
                );
                if fz * fz + (fy * fy + fx * fx) <= squared
                    && coverage
                        .get((strides.0 * x + strides.1 * y + z) as usize)
                        .is_some_and(|&n| n < 2)
                {
                    return false;
                }
            }
        }
    }
    true
}

/// Where the extraction puts its spheres: one cell per axis as it takes
/// it, `(half / n) · 2`, the longest axis's cell (the corner spheres'
/// radius) and the corner spheres' offset per half cell.
#[derive(Debug, Clone, Copy)]
struct Placement {
    step: Vec3,
    cell: f32,
    offset: f32,
}

impl Placement {
    /// The extraction's placement; `scaled` multiplies the offset by the
    /// map's scale as the extraction does (the fallback does not).
    fn of(map: &VoxelMap, scaled: bool) -> Self {
        let counts = counts_of(map);
        let h = map.half_extents;
        let step = Vec3::new(
            h.x / counts[0] as f32 * 2.0,
            h.y / counts[1] as f32 * 2.0,
            h.z / counts[2] as f32 * 2.0,
        );
        let mut axis = 0;
        for i in 1..3 {
            if h[axis] < h[i] {
                axis = i;
            }
        }
        let half_cell = h[axis] / counts[axis] as f32;
        let offset = half_cell * map.resolution;
        Self {
            step,
            cell: half_cell * 2.0,
            offset: if scaled { offset * map.scale } else { offset },
        }
    }
}

/// The original's sphere extraction (the builders run it after the corner
/// masks):
///
/// * candidates: every cell 3 or more from the grid's ends whose eight
///   corners are filled, with the radius (in cells) to the nearest cell
///   with fewer than eight, searched within its distance to the grid's
///   ends; kept when at least 1;
/// * each candidate covers the cells within `r − ¼` of its corner point;
///   the candidates are sorted by radius ([`sort_by_key`]) and, smallest
///   first, any whose every cell another covers too is dropped and its
///   coverage taken back (the largest is left out of this when it is
///   strictly the largest);
/// * the survivors cover again within `r + ½`, and become spheres at their
///   corner point with radius `r/(1 + 0.01r)` longest-axis cells, in
///   double precision;
/// * then each cell with more than `min_corners` corners whose corners the
///   spheres do not cover (at least two left) gives a corner sphere
///   ([`corner_sphere`]).
///
/// With no candidate at all the map takes [`fallback_spheres`] instead.
pub fn extract_spheres(map: &mut VoxelMap, options: &VolumeOptions) {
    map.set_spheres(Vec::new());
    let counts = counts_of(map);
    if map.cells().is_empty() {
        return;
    }
    let strides = strides(counts);
    let (sx, sy) = strides;
    let fill = map.fill();
    let corners = |at: i32| fill.get(at as usize).map_or(0, |f| f[0]);
    let mut candidates = Vec::new();
    for i in 3..=counts[0] - 3 {
        let ei = (counts[0] - i).min(i);
        for j in 3..=counts[1] - 3 {
            let ej = (counts[1] - j).min(j).min(ei);
            for k in 3..=counts[2] - 3 {
                if corners(sx * i + sy * j + k) < 8 {
                    continue;
                }
                let e = (counts[2] - k).min(k).min(ej);
                let mut squared = (e * e) as f32;
                for x in i - e..=i + e {
                    for y in j - e..=j + e {
                        for z in k - e..=k + e {
                            if corners(sx * x + sy * y + z) < 8 {
                                let (dx, dy, dz) = (x - i, y - j, z - k);
                                let d = (dz * dz + dx * dx + dy * dy) as f32;
                                if squared > d {
                                    squared = d;
                                }
                            }
                        }
                    }
                }
                if squared >= 1.0 {
                    candidates.push(Candidate {
                        cell: [i, j, k],
                        radius: squared.sqrt(),
                    });
                }
            }
        }
    }
    if candidates.is_empty() {
        fallback_spheres(map, options);
        return;
    }
    let mut coverage = vec![0i32; map.cells().len()];
    for &c in &candidates {
        cover(&mut coverage, strides, c, c.radius - 0.25, 1);
    }
    let mut keys: Vec<(f32, u32)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.radius, i as u32))
        .collect();
    sort_by_key(&mut keys);
    let mut sorted: Vec<Candidate> = keys.iter().map(|&(_, i)| candidates[i as usize]).collect();
    // The strictly largest candidate is never pruned.
    let last = sorted.len() - 1;
    let prunable = if last > 0 && sorted[last - 1].radius < sorted[last].radius {
        last
    } else {
        last + 1
    };
    for c in sorted.iter_mut().take(prunable) {
        if redundant(&coverage, strides, *c) {
            // The original clears the radius first, but takes the coverage
            // back with the window and limit its radius gave before.
            cover(&mut coverage, strides, *c, c.radius - 0.25, -1);
            c.radius = 0.0;
        }
    }
    for &c in &sorted {
        if c.radius > 0.0 {
            cover(&mut coverage, strides, c, c.radius + 0.5, 1);
        }
    }
    let place = Placement::of(map, true);
    let step = place.step;
    let min = map.centre - map.half_extents;
    let mut spheres = Vec::new();
    for c in sorted.iter().filter(|c| c.radius > 0.0) {
        let r = f64::from(c.radius);
        spheres.push(Sphere {
            centre: Vec3::new(
                c.cell[0] as f32 * step.x + min.x,
                c.cell[1] as f32 * step.y + min.y,
                c.cell[2] as f32 * step.z + min.z,
            ),
            radius: (r / (r * 0.01 + 1.0) * f64::from(place.cell)) as f32,
        });
    }
    for i in 1..=counts[0] - 1 {
        for j in 1..=counts[1] - 1 {
            for k in 1..=counts[2] - 1 {
                let at = sx * i + sy * j + k;
                let [count, mask] = map.fill().get(at as usize).copied().unwrap_or([0, 0]);
                if i32::from(count) <= options.min_corners {
                    continue;
                }
                let mut left = i32::from(count);
                let mut free = mask;
                for (bit, [x, y, z]) in CORNER_CELLS.iter().enumerate() {
                    let corner = at + sx * x + sy * y + z;
                    if coverage.get(corner as usize).is_some_and(|&n| n > 0) {
                        left -= 1;
                        free &= !(1 << bit);
                    }
                }
                if left < 2 {
                    continue;
                }
                if let Some(s) = corner_sphere(map, [i, j, k], free, options, false, place) {
                    spheres.push(s);
                }
            }
        }
    }
    map.set_spheres(spheres);
}

/// The original's corner sphere of a surface cell, shared by the extraction
/// and the fallback: the corners still `free` lose those a neighbour with
/// more than one more filled corner also has ([`shared_corners`]); if any
/// remain, the sphere needs more of them than `min_free_corners`; if none
/// do, only a cell of 4 to 6 corners whose mask is its own mirror, or a
/// face neighbour's, gives one (compared against its corner count in the
/// extraction and against 8 in the fallback, as the original does). The
/// sphere is one longest-axis cell across, at the cell's corner point
/// moved toward its empty side: `−4` half cells per empty corner and the
/// pull of every neighbour short of eight corners, times the offset.
fn corner_sphere(
    map: &VoxelMap,
    at: [i32; 3],
    free: u8,
    options: &VolumeOptions,
    fallback: bool,
    place: Placement,
) -> Option<Sphere> {
    let (sx, sy) = strides(counts_of(map));
    let index = |[x, y, z]: [i32; 3]| sx * x + sy * y + z;
    let fill = map.fill();
    let fill_at = |c: [i32; 3]| fill.get(index(c) as usize).copied().unwrap_or([0, 0]);
    let [count, mask] = fill_at(at);
    let mut free = free;
    for d in NEIGHBOURS {
        let [ncount, nmask] = fill_at([at[0] + d[0], at[1] + d[1], at[2] + d[2]]);
        if i64::from(ncount) - i64::from(count) > 1 {
            free &= !shared_corners(nmask, d);
        }
    }
    let corners = if free == 0 {
        if !(4..7).contains(&count) {
            return None;
        }
        let mirrored = mirror_mask(mask);
        let repeats = mask == mirrored
            || FACE_NEIGHBOURS
                .iter()
                .any(|d| fill_at([at[0] + d[0], at[1] + d[1], at[2] + d[2]])[1] == mirrored);
        if !repeats {
            return None;
        }
        if fallback { 8 } else { i32::from(count) }
    } else {
        free.count_ones() as i32
    };
    if corners <= options.min_free_corners {
        return None;
    }
    let mut dir = [0i32; 3];
    for (bit, d) in CORNER_DIRECTIONS.iter().enumerate() {
        if mask & (1 << bit) == 0 {
            for a in 0..3 {
                dir[a] -= d[a] * 4;
            }
        }
    }
    for d in NEIGHBOURS {
        let n = i32::from(fill_at([at[0] + d[0], at[1] + d[1], at[2] + d[2]])[0]);
        if n < 8 {
            for a in 0..3 {
                dir[a] -= (8 - n) * d[a];
            }
        }
    }
    let min = map.centre - map.half_extents;
    let at_axis = |a: usize| dir[a] as f32 * place.offset + (at[a] as f32 * place.step[a] + min[a]);
    Some(Sphere {
        centre: Vec3::new(at_axis(0), at_axis(1), at_axis(2)),
        radius: place.cell,
    })
}

/// The original's fallback for a mesh with no candidate sphere: the corner
/// spheres of every cell with more than `min_corners` corners, with no
/// coverage to take corners away.
pub fn fallback_spheres(map: &mut VoxelMap, options: &VolumeOptions) {
    let counts = counts_of(map);
    let (sx, sy) = strides(counts);
    let place = Placement::of(map, false);
    let mut spheres = Vec::new();
    for i in 1..=counts[0] - 1 {
        for j in 1..=counts[1] - 1 {
            for k in 1..=counts[2] - 1 {
                let at = sx * i + sy * j + k;
                let [count, mask] = map.fill().get(at as usize).copied().unwrap_or([0, 0]);
                if i32::from(count) <= options.min_corners {
                    continue;
                }
                if let Some(s) = corner_sphere(map, [i, j, k], mask, options, true, place) {
                    spheres.push(s);
                }
            }
        }
    }
    map.set_spheres(spheres);
}

#[cfg(test)]
mod tests;
