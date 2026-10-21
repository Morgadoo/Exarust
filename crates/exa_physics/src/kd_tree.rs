//! The original's triangle kd-tree (`TkdTree` and its terrain subclass
//! `TTErrainkdTree`): the node layout, the build, and the queries that
//! physics and picking run against a mesh.
//!
//! # Layout
//!
//! A node is two words. The first is the leaf flag plus a triangle count, or
//! the split axis in the low two bits plus the byte distance to the first
//! child. The second is the split plane (a float) for an internal node, or
//! an INTEGER index into a separate leaf array for a leaf. The two children
//! are consecutive nodes, the left one first. The leaf array only ever holds
//! the final leaves' triangle indices, in the order the leaves were made.
//!
//! # Build
//!
//! The root cell is the mesh box grown by 0.01 on every side. A stack of
//! `(node, cell)` work items is popped (the right child first):
//!
//! - the root's triangles are bounded by their vertices; every later cell
//!   bounds each triangle by the polygon left after clipping it to the cell;
//! - at most 512 triangles: 15 planes per allowed axis (the cell's extent in
//!   steps of 1/16) are costed against the cell's surface area, and the cell
//!   splits only when the best cost is at most the triangle count;
//! - more than 512: it splits at the middle of the cell's longest axis;
//! - a child is queued only when it holds more than 8 triangles and its cell
//!   is at least 0.1 across; otherwise it becomes a leaf.
//!
//! # Queries
//!
//! `ray`, `segment` and `ray_any` walk the tree front to back with the
//! original's position compares and stop at the first leaf (in walk order)
//! that produced a hit. `sweep` and `sweep_nearest` are the swept-sphere
//! walks; the mesh-side test of a leaf lives in the mesh class in the
//! original, so the leaf test is a caller-supplied closure here.
//!
//! # Assumptions
//!
//! - The mesh keeps a stored plane per triangle that is not part of this
//!   tree's data. [`Triangle::new`] derives it from the winding with the
//!   orientation the ray queries agree on (the normal pointing away from a
//!   ray that hits the front face). The magnitude does not matter.
//! - The root box of a standalone mesh tree is the vertex bounds' centre and
//!   half extent (the original reads them from the mesh).
//! - Lengths are metres here; the original's terrain trees work in
//!   centimetres (see [`TerrainKdTree`]).
//!
//! The running game still uses rapier's trimesh broad phase for the level;
//! this module is not on that path.

#![allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_range_contains,
    clippy::nonminimal_bool
)]

use bevy::math::Vec3;

/// The leaf marker in a node's first word.
pub const LEAF: u32 = 0x8000_0000;
/// A child with more triangles than this keeps splitting.
pub const LEAF_TRIANGLES: u32 = 8;
/// A child whose cell is less than this (m) across becomes a leaf.
pub const LEAF_EXTENT: f64 = 0.1;
/// Triangle counts above this split at the middle without costing planes.
pub const COST_TRIANGLES: u32 = 0x200;
/// The candidate plane count per axis (steps of 1/16 of the extent).
pub const SPLIT_STEPS: u32 = 15;
/// The step (of 16) of the middle split.
pub const MIDDLE_STEP: u32 = 8;
/// An axis whose cell extent is below this (m) is not costed.
pub const MIN_AXIS_EXTENT: f64 = 0.0001;
/// The test epsilon of the ray and segment queries (an `f32`).
pub const EPSILON: f32 = f32::from_bits(0x322b_cc77);
/// The starting best cost of the split search (an `f32` just under the
/// largest finite value, as the original stores it).
pub const BEST_COST: f32 = f32::from_bits(0x7f7f_c99e);
/// The padding added to every side of the root cell.
pub const BOX_PADDING: f32 = f32::from_bits(0x3c23_d70a);
/// The inverse direction the sweeps use for a (nearly) zero component.
pub const FAR: f32 = f32::from_bits(0x4974_2400);
/// A sweep direction component at or below this magnitude counts as zero.
pub const SWEEP_TINY: f32 = 1.192_089_982_282_595_9e-7;

/// The original's dot product: the sum runs z, then y, then x.
fn dot(a: Vec3, b: Vec3) -> f32 {
    a.z * b.z + (a.y * b.y + a.x * b.x)
}

/// One triangle of the queried mesh: its vertices in the mesh's space and
/// the plane the mesh stores with it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    /// The stored plane normal (not normalised).
    pub normal: Vec3,
    /// The stored plane offset: `normal · x = offset` on the triangle.
    pub offset: f32,
}

impl Triangle {
    /// A triangle with the plane derived from its winding: the normal is
    /// `(c - a) × (b - a)`, the direction a front-facing ray travels, and
    /// the offset is its dot with `a`.
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let normal = (c - a).cross(b - a);
        Self {
            vertices: [a, b, c],
            normal,
            offset: dot(normal, a),
        }
    }

    /// The triangle's axis-aligned bounds, folded as the original folds a
    /// point list (start at the first vertex, widen with the others).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        fold_bounds(&self.vertices)
    }
}

/// Bounds of a point list, starting from its first point. An empty list
/// yields a degenerate box at the origin.
fn fold_bounds(points: &[Vec3]) -> (Vec3, Vec3) {
    let first = points.first().copied().unwrap_or(Vec3::ZERO);
    let (mut min, mut max) = (first, first);
    for p in points.iter().skip(1) {
        for axis in 0..3 {
            let v = p[axis];
            if min[axis] <= v {
                if max[axis] < v {
                    max[axis] = v;
                }
            } else {
                min[axis] = v;
            }
        }
    }
    (min, max)
}

/// One tree node, the two words the original keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub word: u32,
    /// The split plane for an internal node. For a leaf the bits hold the
    /// integer index of its first triangle in the leaf array (read it with
    /// [`Node::triangle_start`]).
    pub value: f32,
}

impl Node {
    /// A leaf of `count` triangles starting at `start` in the leaf array.
    pub fn leaf(count: u32, start: u32) -> Self {
        Self {
            word: count | LEAF,
            value: f32::from_bits(start),
        }
    }

    /// An internal node on `axis` whose first child is `first_child` nodes
    /// after it.
    pub fn split(axis: usize, first_child: usize, plane: f32) -> Self {
        Self {
            word: ((first_child * 8) as u32 & 0x7fff_fffc) | axis as u32,
            value: plane,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.word & LEAF != 0
    }

    /// The split axis (0, 1 or 2).
    pub fn axis(&self) -> usize {
        (self.word & 3) as usize
    }

    /// The first child's byte offset from this node.
    pub fn child_offset(&self) -> usize {
        (self.word & 0x7fff_fffc) as usize
    }

    /// A leaf's triangle count.
    pub fn triangle_count(&self) -> u32 {
        self.word & 0x7fff_ffff
    }

    /// A leaf's first index in the leaf array.
    pub fn triangle_start(&self) -> usize {
        self.value.to_bits() as usize
    }
}

/// A hit of the ray and segment queries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The triangle's index in the mesh.
    pub triangle: u32,
    /// The distance along the query direction.
    pub distance: f32,
}

/// A swept sphere: `radius` around a centre moving `origin + t·direction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sweep {
    pub origin: Vec3,
    pub direction: Vec3,
    pub radius: f32,
}

/// A deferred far child: the node, the distance its interval starts at, and
/// the interval end of the node that was split.
#[derive(Clone, Copy)]
struct Frame {
    node: usize,
    t: f32,
    saved: f32,
}

/// The walk state: the current node, the distance interval, and the stack.
struct Walk {
    node: usize,
    tmin: f32,
    tmax: f32,
    stack: Vec<Frame>,
}

impl Walk {
    fn new(tmin: f32, tmax: f32) -> Self {
        Self {
            node: 0,
            tmin,
            tmax,
            stack: Vec::new(),
        }
    }

    /// Resumes at the most recent deferred child. False when none is left.
    fn pop(&mut self) -> bool {
        match self.stack.pop() {
            Some(f) => {
                self.node = f.node;
                self.tmin = f.t;
                self.tmax = f.saved;
                true
            }
            None => false,
        }
    }
}

/// A triangle kd-tree.
#[derive(Clone, Debug, Default)]
pub struct KdTree {
    pub nodes: Vec<Node>,
    /// The leaf array: triangle indices, leaf by leaf in creation order.
    pub triangles: Vec<u32>,
    /// The triangle set the tree was built from.
    pub mesh: Vec<Triangle>,
    /// The root cell (the mesh box plus [`BOX_PADDING`]).
    pub min: Vec3,
    pub max: Vec3,
    /// The original's build flag: no split along the y axis.
    pub flat: bool,
}

/// One cell waiting to be split.
struct Work {
    node: usize,
    min: Vec3,
    max: Vec3,
    list: Vec<u32>,
}

impl KdTree {
    /// Builds a tree over `mesh`. `flat` forbids splitting along y, as the
    /// original's terrain-style trees do; a mesh tree passes `false`. An
    /// empty mesh gives an empty tree (the original logs an error).
    pub fn build(mesh: &[Triangle], flat: bool) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            triangles: Vec::new(),
            mesh: mesh.to_vec(),
            min: Vec3::ZERO,
            max: Vec3::ZERO,
            flat,
        };
        if mesh.is_empty() {
            return tree;
        }
        let points: Vec<Vec3> = mesh.iter().flat_map(|t| t.vertices).collect();
        let (lo, hi) = fold_bounds(&points);
        let centre = (lo + hi) * 0.5;
        let half = (hi - lo) * 0.5;
        tree.min = centre - half - Vec3::splat(BOX_PADDING);
        tree.max = centre + half + Vec3::splat(BOX_PADDING);

        let root_count = mesh.len() as u32;
        tree.nodes.push(Node::leaf(root_count, 0));
        let mut stack = vec![Work {
            node: 0,
            min: tree.min,
            max: tree.max,
            list: (0..root_count).collect(),
        }];
        let mut first = true;
        while let Some(item) = stack.pop() {
            // Bounds per list position. The root has no cell to clip to.
            let bounds: Vec<(Vec3, Vec3)> = item
                .list
                .iter()
                .map(|&id| {
                    let t = &mesh[id as usize];
                    if first {
                        t.bounds()
                    } else {
                        clipped_bounds(&t.vertices, item.min, item.max)
                    }
                })
                .collect();
            first = false;
            let Some((axis, plane)) = choose_split(&bounds, item.min, item.max, flat) else {
                tree.finish_leaf(item.node, &item.list);
                continue;
            };
            let (mut left, mut right) = (Vec::new(), Vec::new());
            for (&id, (lo, hi)) in item.list.iter().zip(&bounds) {
                if plane < lo[axis] {
                    right.push(id);
                } else {
                    left.push(id);
                    if plane <= hi[axis] {
                        right.push(id);
                    }
                }
            }
            let first_child = tree.nodes.len();
            tree.nodes.push(Node::leaf(left.len() as u32, 0));
            tree.nodes.push(Node::leaf(right.len() as u32, 0));
            tree.nodes[item.node] = Node::split(axis, first_child - item.node, plane);

            let mut left_max = item.max;
            left_max[axis] = plane;
            let mut right_min = item.min;
            right_min[axis] = plane;
            for (node, min, max, list) in [
                (first_child, item.min, left_max, left),
                (first_child + 1, right_min, item.max, right),
            ] {
                let extent = {
                    let e = max - min;
                    let mut longest = e.x;
                    if longest < e.y {
                        longest = e.y;
                    }
                    if longest < e.z {
                        longest = e.z;
                    }
                    longest
                };
                if list.len() as u32 > LEAF_TRIANGLES && !(f64::from(extent) < LEAF_EXTENT) {
                    stack.push(Work {
                        node,
                        min,
                        max,
                        list,
                    });
                } else if !list.is_empty() {
                    tree.finish_leaf(node, &list);
                }
            }
        }
        tree
    }

    /// Writes `list` to the leaf array and points the node at it.
    fn finish_leaf(&mut self, node: usize, list: &[u32]) {
        let start = self.triangles.len() as u32;
        self.triangles.extend_from_slice(list);
        self.nodes[node] = Node::leaf(list.len() as u32, start);
    }

    /// The triangle indices of a leaf.
    fn leaf_triangles(&self, node: Node) -> &[u32] {
        let start = node.triangle_start();
        &self.triangles[start..start + node.triangle_count() as usize]
    }

    /// Descends from the walk's node to a leaf, deferring the far child
    /// whenever the ray crosses a split plane inside the current interval.
    fn descend(&self, w: &mut Walk, origin: Vec3, dir: Vec3, inv: Vec3) {
        loop {
            let node = self.nodes[w.node];
            if node.is_leaf() {
                return;
            }
            let a = node.axis();
            let plane = node.value;
            let left = w.node + node.child_offset() / 8;
            let right = left + 1;
            let p0 = dir[a] * w.tmin + origin[a];
            let p1 = dir[a] * w.tmax + origin[a];
            if p0 > plane {
                if p1 <= plane {
                    let t = (plane - origin[a]) * inv[a];
                    w.stack.push(Frame {
                        node: left,
                        t,
                        saved: w.tmax,
                    });
                    w.tmax = t;
                }
                w.node = right;
            } else {
                if plane < p1 {
                    let t = (plane - origin[a]) * inv[a];
                    w.stack.push(Frame {
                        node: right,
                        t,
                        saved: w.tmax,
                    });
                    w.tmax = t;
                }
                w.node = left;
            }
        }
    }

    /// Clips the ray to the root cell: the starting interval and the
    /// per-axis inverse direction, or `None` when it misses.
    fn enter(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<(f32, f32, Vec3)> {
        let mut tmin = 0.0f32;
        let mut tmax = max;
        let mut inv = Vec3::ZERO;
        for a in 0..3 {
            let (lo, hi) = (self.min[a], self.max[a]);
            let p0 = dir[a] * tmin + origin[a];
            let p1 = dir[a] * tmax + origin[a];
            if lo <= p0 {
                if p0 <= hi {
                    inv[a] = 1.0 / dir[a];
                    if lo <= p1 {
                        if hi < p1 {
                            tmax = (hi - origin[a]) * inv[a];
                        }
                    } else {
                        tmax = (lo - origin[a]) * inv[a];
                    }
                } else {
                    if hi <= p1 {
                        return None;
                    }
                    inv[a] = 1.0 / dir[a];
                    if p1 < lo {
                        tmax = (lo - origin[a]) * inv[a];
                    }
                    tmin = (hi - origin[a]) * inv[a];
                }
            } else {
                if p1 <= lo {
                    return None;
                }
                inv[a] = 1.0 / dir[a];
                if hi < p1 {
                    tmax = (hi - origin[a]) * inv[a];
                }
                tmin = (lo - origin[a]) * inv[a];
            }
            if tmax < tmin {
                return None;
            }
        }
        Some((tmin, tmax, inv))
    }

    /// The first front-face hit along a ray `origin + t·direction`,
    /// `0 < t < max`. The walk stops at the first leaf that has a hit, and a
    /// hit must lie inside that leaf's part of the ray, so a triangle
    /// reaching into a later leaf can be passed over.
    pub fn ray(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<Hit> {
        self.find(origin, direction, max, ray_triangle)
    }

    /// The first hit along a segment `origin + t·delta`, `0 < t <= max`,
    /// against both faces. The far end is included.
    pub fn segment(&self, origin: Vec3, delta: Vec3, max: f32) -> Option<Hit> {
        self.find(origin, delta, max, segment_triangle)
    }

    fn find(
        &self,
        origin: Vec3,
        dir: Vec3,
        max: f32,
        test: fn(&Triangle, Vec3, Vec3, f32) -> Option<f32>,
    ) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let (tmin, tmax, inv) = self.enter(origin, dir, max)?;
        let mut w = Walk::new(tmin, tmax);
        loop {
            self.descend(&mut w, origin, dir, inv);
            let leaf = self.nodes[w.node];
            let mut found = None;
            for &id in self.leaf_triangles(leaf) {
                if let Some(t) = test(&self.mesh[id as usize], origin, dir, w.tmax) {
                    found = Some(id);
                    w.tmax = t;
                }
            }
            if let Some(triangle) = found {
                return Some(Hit {
                    triangle,
                    distance: w.tmax,
                });
            }
            if !w.pop() {
                return None;
            }
        }
    }

    /// The first triangle a ray meets, in walk order, within `max`: the
    /// original's boolean ray query. There is no box clip; the walk starts
    /// at the root over `[0, max]`.
    pub fn ray_any(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<u32> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut inv = Vec3::ONE;
        for a in 0..3 {
            if direction[a].abs() > 0.0 {
                inv[a] = 1.0 / direction[a];
            }
        }
        let mut w = Walk::new(0.0, max);
        loop {
            self.descend(&mut w, origin, direction, inv);
            let leaf = self.nodes[w.node];
            for &id in self.leaf_triangles(leaf) {
                if ray_any_triangle(&self.mesh[id as usize], origin, direction, w.tmax) {
                    return Some(id);
                }
            }
            if !w.pop() {
                return None;
            }
        }
    }

    /// Descends a swept sphere to its next leaf. The sphere's radius widens
    /// every plane test, so both children are visited near a split.
    fn descend_sweep(&self, w: &mut Walk, s: &Sweep, inv: Vec3) {
        loop {
            let node = self.nodes[w.node];
            if node.is_leaf() {
                return;
            }
            let a = node.axis();
            let plane = node.value;
            let left = w.node + node.child_offset() / 8;
            let right = left + 1;
            let (d, o, r) = (s.direction[a], s.origin[a], s.radius);
            let p0 = d * w.tmin + o;
            let p1 = d * w.tmax + o;
            if 0.0 <= d {
                if p0 - r <= plane {
                    if plane <= p1 + r {
                        let t0 = ((plane - o) - r) * inv[a];
                        let t = if t0 <= w.tmin { w.tmin } else { t0 };
                        w.stack.push(Frame {
                            node: right,
                            t,
                            saved: w.tmax,
                        });
                        let t2 = ((plane - o) + r) * inv[a];
                        if t2 < w.tmax {
                            w.tmax = t2;
                        }
                    }
                    w.node = left;
                } else {
                    w.node = right;
                }
            } else if p1 - r <= plane {
                if plane <= p0 + r {
                    let t0 = ((plane - o) + r) * inv[a];
                    let t = if t0 <= w.tmin { w.tmin } else { t0 };
                    w.stack.push(Frame {
                        node: left,
                        t,
                        saved: w.tmax,
                    });
                    let t2 = ((plane - o) - r) * inv[a];
                    if t2 < w.tmax {
                        w.tmax = t2;
                    }
                    w.node = right;
                } else {
                    w.node = left;
                }
            } else {
                w.node = right;
            }
        }
    }

    /// Walks every leaf a swept sphere reaches (the boolean sweeps). `leaf`
    /// is the mesh-side test of one leaf: it receives the sweep, the leaf's
    /// triangle indices and the distance limit, which it lowers to the
    /// distance of a hit it reports by returning true. Returns the final
    /// limit when any leaf hit. The limit starts at `max`, and the walk
    /// never goes past the best hit found so far.
    pub fn sweep(
        &self,
        sweep: &Sweep,
        max: f32,
        mut leaf: impl FnMut(&Sweep, &[u32], &mut f32) -> bool,
    ) -> Option<f32> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = sweep_inverse(sweep.direction, false);
        let mut best = max;
        let mut hit = false;
        let mut w = Walk::new(0.0, max);
        loop {
            self.descend_sweep(&mut w, sweep, inv);
            w.tmax = best;
            let node = self.nodes[w.node];
            if node.triangle_count() != 0 && leaf(sweep, self.leaf_triangles(node), &mut w.tmax) {
                hit = true;
                best = w.tmax;
            }
            if !w.pop() {
                break;
            }
            if best < w.tmax {
                w.tmax = best;
            }
        }
        hit.then_some(best)
    }

    /// The nearest-triangle sweep: like [`Self::sweep`], but the leaf test
    /// returns the triangle it hit, the last hit wins, and the limit only
    /// follows a hit that is strictly nearer. The returned distance is the
    /// final limit.
    pub fn sweep_nearest(
        &self,
        sweep: &Sweep,
        max: f32,
        mut leaf: impl FnMut(&Sweep, &[u32], &mut f32) -> Option<u32>,
    ) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = sweep_inverse(sweep.direction, true);
        let mut best = max;
        let mut found = None;
        let mut w = Walk::new(0.0, max);
        loop {
            self.descend_sweep(&mut w, sweep, inv);
            w.tmax = best;
            let node = self.nodes[w.node];
            if node.triangle_count() != 0
                && let Some(id) = leaf(sweep, self.leaf_triangles(node), &mut w.tmax)
            {
                found = Some(id);
                if w.tmax < best {
                    best = w.tmax;
                }
            }
            if !w.pop() {
                break;
            }
        }
        found.map(|triangle| Hit {
            triangle,
            distance: best,
        })
    }
}

/// The inverse direction of a sweep: a component at or under
/// [`SWEEP_TINY`] becomes plus or minus [`FAR`]. The nearest sweep keeps the
/// sign of the component; the boolean sweeps always use plus.
fn sweep_inverse(dir: Vec3, signed: bool) -> Vec3 {
    let mut inv = Vec3::ZERO;
    for a in 0..3 {
        let d = dir[a];
        inv[a] = if d.abs() <= SWEEP_TINY {
            if signed && !(0.0 <= d) { -FAR } else { FAR }
        } else {
            1.0 / d
        };
    }
    inv
}

/// Bounds of `triangle` clipped to the cell: the clipped polygon's vertex
/// bounds. A triangle that leaves nothing keeps the stale first vertex of
/// the last non-empty polygon, as the original's scratch buffer does.
fn clipped_bounds(triangle: &[Vec3; 3], min: Vec3, max: Vec3) -> (Vec3, Vec3) {
    const CAP: usize = 16;
    let mut p = [Vec3::ZERO; CAP];
    p[..3].copy_from_slice(triangle);
    let mut count = 3usize;
    let mut e = [Vec3::ZERO; CAP];
    for axis in 0..3 {
        if count == 0 {
            break;
        }
        let mid = clip_plane(&p[..count], &mut e, axis, min[axis], true);
        if mid == 0 {
            count = 0;
            break;
        }
        count = clip_plane(&e[..mid], &mut p, axis, max[axis], false);
    }
    fold_bounds(&p[..count.max(1)])
}

/// One Sutherland-Hodgman pass against `plane` on `axis`, keeping the
/// coordinates at or above it (`lower`) or at or below it. Edges run from
/// each vertex's predecessor to the vertex; a vertex exactly on the plane is
/// kept without an intersection. Returns the output count.
fn clip_plane(src: &[Vec3], dst: &mut [Vec3], axis: usize, plane: f32, lower: bool) -> usize {
    let len = src.len();
    let mut n = 0;
    for i in 0..len {
        let b = src[i];
        let a = src[(i + len - 1) % len];
        let (ac, bc) = (a[axis], b[axis]);
        let (inside, enters, leaves) = if lower {
            (bc >= plane, ac < plane && bc > plane, ac > plane)
        } else {
            (bc <= plane, ac > plane && bc < plane, ac < plane)
        };
        let crossing = if inside { enters } else { leaves };
        if crossing {
            let t = (plane - ac) / (bc - ac);
            dst[n] = a + (b - a) * t;
            n += 1;
        }
        if inside {
            dst[n] = b;
            n += 1;
        }
    }
    n
}

/// The candidate plane at `step`/16 of the cell: formed in double precision
/// from the single-precision extent, as the original forms it.
fn plane_at(min: f32, max: f32, step: u32) -> f32 {
    (f64::from(min) + (f64::from(step) * 0.0625) * f64::from(max - min)) as f32
}

/// The original's split choice for a cell: the axis and plane, or `None`
/// for a leaf. `bounds` are the triangles' bounds, one per list position.
fn choose_split(bounds: &[(Vec3, Vec3)], min: Vec3, max: Vec3, flat: bool) -> Option<(usize, f32)> {
    let count = bounds.len();
    let ext = max - min;
    if count as u32 > COST_TRIANGLES {
        let mut axis = 0;
        for cand in 1..3 {
            if flat && cand == 1 {
                continue;
            }
            if ext[axis] < ext[cand] {
                axis = cand;
            }
        }
        return Some((axis, plane_at(min[axis], max[axis], MIDDLE_STEP)));
    }
    let mut best = BEST_COST;
    let mut choice = None;
    for a in 0..3 {
        if flat && a == 1 {
            continue;
        }
        if f64::from(ext[a]) < MIN_AXIS_EXTENT {
            continue;
        }
        let e1 = ext[(a + 1) % 3];
        let e2 = ext[(a + 2) % 3];
        let x8 = e2 * e1;
        let x6 = e1 + e2;
        let inv = 1.0 / (ext[a] * x6 + x8);
        for step in 1..=SPLIT_STEPS {
            let s = plane_at(min[a], max[a], step);
            let (mut c0, mut c1) = (0u32, 0u32);
            for (lo, hi) in bounds {
                if s < lo[a] {
                    c1 += 1;
                } else {
                    c0 += 1;
                    if s <= hi[a] {
                        c1 += 1;
                    }
                }
            }
            let area_left = (((s - min[a]) * x6) + x8) * inv;
            let area_right = (((max[a] - s) * x6) + x8) * inv;
            let cost = ((c1 as f32 * area_right) + (c0 as f32 * area_left)) + 1.0;
            if cost < best {
                best = cost;
                choice = Some((a, s));
            }
        }
    }
    if best <= count as f32 { choice } else { None }
}

/// The nearest-hit triangle test: front faces only, the hit strictly inside
/// `(0, tmax)`.
fn ray_triangle(tri: &Triangle, origin: Vec3, dir: Vec3, tmax: f32) -> Option<f32> {
    let [v0, v1, v2] = tri.vertices;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let p = dir.cross(e2);
    let det = dot(p, e1);
    if det < EPSILON {
        return None;
    }
    let t = origin - v0;
    let u = dot(t, p);
    if !(0.0 <= u && u <= det) {
        return None;
    }
    let q = t.cross(e1);
    let v = dot(dir, q);
    if !(0.0 <= v && u + v <= det) {
        return None;
    }
    let dist = dot(e2, q) / det;
    (0.0 < dist && dist < tmax).then_some(dist)
}

/// The segment test: both faces, the hit inside `(0, tmax]`.
fn segment_triangle(tri: &Triangle, origin: Vec3, dir: Vec3, tmax: f32) -> Option<f32> {
    let [v0, v1, v2] = tri.vertices;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let p = dir.cross(e2);
    let det = dot(p, e1);
    if det.abs() < EPSILON {
        return None;
    }
    let inv = 1.0 / det;
    let t = origin - v0;
    let u = dot(t, p) * inv;
    if !(0.0 <= u && u <= 1.0) {
        return None;
    }
    let q = t.cross(e1);
    let v = dot(dir, q) * inv;
    if !(0.0 <= v && u + v <= 1.0) {
        return None;
    }
    let dist = dot(e2, q) * inv;
    (0.0 < dist && dist <= tmax).then_some(dist)
}

/// The boolean ray test against the stored plane: hit the plane, then test
/// the point in the triangle's projection onto the two axes after the
/// normal's dominant one.
fn ray_any_triangle(tri: &Triangle, origin: Vec3, dir: Vec3, tmax: f32) -> bool {
    let n = tri.normal;
    let den = dot(dir, n);
    if den < EPSILON {
        return false;
    }
    let t = -(dot(origin, n) - tri.offset) / den;
    if t < EPSILON || t > tmax {
        return false;
    }
    let (n0, n1, n2) = (n.x.abs(), n.y.abs(), n.z.abs());
    let a = if !(n1 < n0) {
        if n2 < n1 { 2 } else { 0 }
    } else if n2 < n0 {
        1
    } else {
        0
    };
    let b = [1usize, 2, 0][a];
    let [v0, v1, v2] = tri.vertices;
    let e1a = v1[a] - v0[a];
    let e1b = v1[b] - v0[b];
    let e2a = v2[a] - v0[a];
    let e2b = v2[b] - v0[b];
    let inv = 1.0 / (e2a * e1b - e2b * e1a);
    let pa = ((dir[a] * t + origin[a]) - v0[a]) * inv;
    let pb = ((dir[b] * t + origin[b]) - v0[b]) * inv;
    let w1 = pb * e2a - pa * e2b;
    let w2 = pa * e1b - pb * e1a;
    !(w1 < 0.0) && !(w2 < 0.0) && w1 + w2 <= 1.0
}

/// A terrain block's tree (`TTErrainkdTree`): the cell grid of one block,
/// halved along x or z until a node covers fewer than 4 x 4 cells. A leaf
/// lists two triangles per cell.
///
/// A node's second word holds the integer cell range until it is split:
/// four bytes, min x, max x, min z, max z. The original's trees work in
/// terrain units where one cell is 20 units (centimetres); this port stores
/// metres, so the spacing is 0.2.
///
/// The original clones a block's tree as a view that shares the node and
/// leaf arrays with the source and moves the planes in place; the port's
/// [`Self::translated`] copies the arrays instead.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainKdTree {
    pub nodes: Vec<Node>,
    /// The leaf array: two triangles per cell, leaf by leaf.
    pub triangles: Vec<u32>,
    /// Cells along a side (30 for a 6 m block of 20 cm cells).
    pub cells: usize,
    /// The cell size (0.2 m).
    pub spacing: f32,
    /// The cached bounds, set by [`Self::attach`]; zero before.
    pub min: Vec3,
    pub max: Vec3,
}

/// Cells along a terrain block's side.
pub const TERRAIN_CELLS: usize = 30;
/// A terrain block's cell size, metres.
pub const TERRAIN_SPACING: f32 = 0.2;
/// A terrain node with fewer cells than this along both sides is a leaf.
pub const TERRAIN_LEAF_CELLS: u8 = 4;

impl Default for TerrainKdTree {
    fn default() -> Self {
        Self::build(TERRAIN_CELLS, TERRAIN_SPACING)
    }
}

impl TerrainKdTree {
    /// Builds the tree over `cells` x `cells` cells (at most 255) of
    /// `spacing`. The root always splits; axis z is chosen only when the
    /// node is longer in z than in x.
    pub fn build(cells: usize, spacing: f32) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            triangles: Vec::new(),
            cells,
            spacing,
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        };
        if cells == 0 {
            return tree;
        }
        let n = cells.min(255) as u8;
        tree.nodes.push(Node {
            word: LEAF,
            value: pack_range([0, n, 0, n]),
        });
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let range = unpack_range(tree.nodes[index].value);
            let dx = range[1] - range[0];
            let dz = range[3] - range[2];
            let axis = if dx < dz { 2usize } else { 0 };
            let (lo, hi) = (range[axis] as u32, range[axis + 1] as u32);
            let mid = ((hi - lo) >> 1) + lo;
            let first = tree.nodes.len();
            tree.nodes.push(tree.nodes[index]);
            tree.nodes.push(tree.nodes[index]);
            let mut left = range;
            left[axis + 1] = mid as u8;
            let mut right = range;
            right[axis] = mid as u8;
            tree.nodes[first].value = pack_range(left);
            tree.nodes[first + 1].value = pack_range(right);
            let plane = (mid as i32 - (n as i32) / 2) as f32 * spacing;
            tree.nodes[index] = Node::split(axis, first - index, plane);
            for child in [first, first + 1] {
                let r = unpack_range(tree.nodes[child].value);
                if r[1] - r[0] < TERRAIN_LEAF_CELLS && r[3] - r[2] < TERRAIN_LEAF_CELLS {
                    tree.make_leaf(child);
                } else {
                    stack.push(child);
                }
            }
        }
        tree
    }

    fn make_leaf(&mut self, node: usize) {
        let r = unpack_range(self.nodes[node].value);
        let start = self.triangles.len() as u32;
        let count = (r[1] - r[0]) as u32 * (r[3] - r[2]) as u32 * 2;
        for z in r[2]..r[3] {
            for x in r[0]..r[1] {
                let cell = x as u32 + self.cells as u32 * z as u32;
                self.triangles.push(cell * 2);
                self.triangles.push(cell * 2 + 1);
            }
        }
        self.nodes[node] = Node::leaf(count, start);
    }

    /// Moves every split plane by `offset`. The cached bounds, the cell
    /// ranges and the triangles stay where they are.
    pub fn translate(&mut self, offset: Vec3) {
        for node in &mut self.nodes {
            if !node.is_leaf() {
                node.value += offset[node.axis()];
            }
        }
    }

    /// A copy with the planes moved by `offset`.
    pub fn translated(&self, offset: Vec3) -> Self {
        let mut tree = self.clone();
        tree.translate(offset);
        tree
    }

    /// Caches the box of the mesh the tree is attached to, from the mesh's
    /// centre and half extent. Unlike [`KdTree`]'s root cell it has no
    /// padding.
    pub fn attach(&mut self, centre: Vec3, half: Vec3) {
        self.min = centre - half;
        self.max = centre + half;
    }

    /// The triangles of a cell: two per cell, index `(x + n·z)·2`.
    pub fn cell_triangles(cell: u32) -> [u32; 2] {
        [cell * 2, cell * 2 + 1]
    }
}

fn pack_range(range: [u8; 4]) -> f32 {
    f32::from_bits(u32::from_le_bytes(range))
}

fn unpack_range(value: f32) -> [u8; 4] {
    value.to_bits().to_le_bytes()
}

#[cfg(test)]
#[path = "kd_tree_test.rs"]
mod tests;
