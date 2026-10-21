//! The original's triangle kd-tree (`TkdTree` and its terrain subclass
//! `TTErrainkdTree`): the split rule, the node layout, and the queries that
//! physics and picking run against a mesh.
//!
//! The tree splits a triangle set into axis-aligned cells. A node is two
//! words: a split axis in the low two bits and the distance to its first
//! child in the next 29 bits, or the leaf marker with a triangle count; the
//! second word is the split plane (internal nodes) or the first triangle
//! index (leaves). Children are two consecutive nodes, the left one first.
//! The split rule is the original's:
//!
//! - a leaf of more than 512 triangles, or one whose best candidate costs
//!   more than its triangle count, splits at the middle of its longest
//!   axis (the y axis only when the caller allows it);
//! - otherwise the 15 planes at steps of 1/16 of each axis' extent are
//!   tried and the one with the lowest estimated cost wins;
//! - a child becomes a leaf when it holds at most 8 triangles, or when its
//!   longest extent is under 10 cm (0.1 m).
//!
//! The queries keep the original's comparisons: the nearest-hit ray test
//! only accepts a front face (its denominator must be at least 1e-8), the
//! segment test accepts both faces (the sign bit is cleared) and includes
//! the far end, and the overlap tests use the tree box grown by the query
//! radius. Lengths are metres.
//!
//! The running game still uses rapier's trimesh broad phase for the level:
//! this module is the original's own tree, tested on its own, not yet on
//! the game path.

use bevy::math::Vec3;

/// The leaf marker in a node's first word.
pub const LEAF: u32 = 0x8000_0000;
/// A child with at most this many triangles is made a leaf.
pub const LEAF_TRIANGLES: u32 = 8;
/// A child whose longest extent is below this (m) is made a leaf.
pub const LEAF_EXTENT: f64 = 0.1;
/// Triangle counts above this split at the middle without costing planes.
pub const COST_TRIANGLES: u32 = 0x200;
/// The split plane count per axis (steps of 1/16 of the extent).
pub const SPLIT_STEPS: u32 = 15;
/// The test epsilon of the ray and segment queries (an `f32`).
pub const EPSILON: f32 = 9.999_999_9e-9;
/// The starting best cost of the split search (an `f32` just under the
/// largest finite value, as the original stores it).
pub const BEST_COST: f32 = f32::from_bits(0x7f7f_c99e);
/// The original's extreme distance for a query direction component that is
/// (almost) zero.
pub const FAR: f32 = 1.0e6;

/// One triangle of the queried mesh, its vertices in the mesh's space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
}

impl Triangle {
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self {
            vertices: [a, b, c],
        }
    }

    /// The triangle's axis-aligned bounds.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let [a, b, c] = self.vertices;
        (a.min(b).min(c), a.max(b).max(c))
    }

    /// The triangle's normal, not normalised (the plane's orientation).
    pub fn normal(&self) -> Vec3 {
        let [a, b, c] = self.vertices;
        (b - a).cross(c - a)
    }

    /// The triangle's plane: `normal · x = offset`.
    pub fn plane(&self) -> (Vec3, f32) {
        let n = self.normal();
        (n, n.dot(self.vertices[0]))
    }
}

/// One tree node, the two words the original keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub word: u32,
    /// The split plane for an internal node; the first triangle index for a
    /// leaf.
    pub value: f32,
}

impl Node {
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

    /// A leaf's first triangle index.
    pub fn triangle_start(&self) -> usize {
        self.value as usize
    }
}

/// A hit of one of the point or ray queries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The triangle's index in the mesh.
    pub triangle: u32,
    /// The distance along the query direction (or the closest distance for
    /// a projection query).
    pub distance: f32,
}

/// A triangle kd-tree.
#[derive(Clone, Debug, Default)]
pub struct KdTree {
    pub nodes: Vec<Node>,
    /// Leaf triangle indices, in leaf order.
    pub triangles: Vec<u32>,
    /// The triangle set the tree was built from.
    pub mesh: Vec<Triangle>,
    /// The tree's bounds.
    pub min: Vec3,
    pub max: Vec3,
    /// The original's build flag: no split along the y axis.
    pub flat: bool,
}

impl KdTree {
    /// Builds a tree over `mesh`. `flat` forbids splitting along y, as the
    /// original's terrain trees do; a mesh tree passes `false`.
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
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for t in mesh {
            for v in t.vertices {
                min = min.min(v);
                max = max.max(v);
            }
        }
        tree.min = min;
        tree.max = max;

        // The root starts as a leaf over every triangle.
        tree.nodes.push(Node {
            word: (mesh.len() as u32) | LEAF,
            value: 0.0,
        });
        let mut work = vec![Work {
            node: 0,
            tris: (0..mesh.len() as u32).collect(),
        }];
        while let Some(item) = work.pop() {
            let count = item.tris.len() as u32;
            let (tmin, tmax) = cell_bounds(&item.tris, mesh);
            let split = choose_split(&item.tris, mesh, tmin, tmax, flat, count);
            let Some((axis, plane)) = split else {
                tree.make_leaf(item.node, &item.tris);
                continue;
            };
            let parent = item.node;
            let mut left = Vec::new();
            let mut right = Vec::new();
            for &id in &item.tris {
                let (bmin, bmax) = mesh[id as usize].bounds();
                let lo = bmin[axis];
                let hi = bmax[axis];
                if plane < lo {
                    right.push(id);
                } else {
                    left.push(id);
                    if plane <= hi {
                        right.push(id);
                    }
                }
            }
            let first = tree.nodes.len() as u32;
            tree.nodes.push(Node {
                word: 0,
                value: 0.0,
            });
            tree.nodes.push(Node {
                word: 0,
                value: 0.0,
            });
            // The pair is appended contiguously; the word is the first
            // child's byte offset from the parent.
            let offset = ((first as usize) - (parent as usize)) * 8;
            tree.nodes[parent as usize] = Node {
                word: offset as u32 | axis as u32,
                value: plane,
            };
            for (tris, node) in [(left, first), (right, first + 1)] {
                let (bmin, bmax) = cell_bounds(&tris, mesh);
                let extent = (bmax - bmin).max_element();
                if tris.len() as u32 > LEAF_TRIANGLES && f64::from(extent) >= LEAF_EXTENT {
                    work.push(Work { node, tris });
                } else {
                    tree.make_leaf(node, &tris);
                }
            }
        }
        tree
    }

    fn make_leaf(&mut self, node: u32, tris: &[u32]) {
        let start = self.triangles.len();
        self.triangles.extend_from_slice(tris);
        self.nodes[node as usize] = Node {
            word: (tris.len() as u32) | LEAF,
            value: start as f32,
        };
    }

    /// The nearest front face along a ray: `origin + t·direction`, with
    /// `0 < t < max` (the original's `tmax` starts at the query's value and
    /// only strictly nearer hits replace it).
    pub fn ray(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<Hit> {
        let mut best = Hit {
            triangle: u32::MAX,
            distance: max,
        };
        let mut stack = Vec::new();
        let (near, far) = self.clip(origin, direction, 0.0, best.distance)?;
        stack.push((0usize, near, far));
        while let Some((index, tmin, tmax)) = stack.pop() {
            let node = self.nodes[index];
            if node.is_leaf() {
                let start = node.triangle_start();
                for &tri in &self.triangles[start..start + node.triangle_count() as usize] {
                    if let Some(t) =
                        ray_triangle(self.mesh[tri as usize], origin, direction, &mut best)
                    {
                        best = Hit {
                            triangle: tri,
                            distance: t,
                        };
                    }
                }
                continue;
            }
            visit_children(&mut stack, index, node, origin, direction, tmin, tmax);
        }
        (best.triangle != u32::MAX).then_some(best)
    }

    /// The nearest hit along a segment `origin + t·delta`, `0 ≤ t ≤ max`,
    /// against both faces. The far end is included (the original compares
    /// `t <= tmax` after normalising by the determinant).
    pub fn segment(&self, origin: Vec3, delta: Vec3, max: f32) -> Option<Hit> {
        let mut best = Hit {
            triangle: u32::MAX,
            distance: max,
        };
        let mut stack = Vec::new();
        let (near, far) = self.clip(origin, delta, 0.0, best.distance)?;
        stack.push((0usize, near, far));
        while let Some((index, tmin, tmax)) = stack.pop() {
            let node = self.nodes[index];
            if node.is_leaf() {
                let start = node.triangle_start();
                for &tri in &self.triangles[start..start + node.triangle_count() as usize] {
                    if let Some(t) =
                        segment_triangle(self.mesh[tri as usize], origin, delta, &mut best)
                    {
                        best = Hit {
                            triangle: tri,
                            distance: t,
                        };
                    }
                }
                continue;
            }
            visit_children(&mut stack, index, node, origin, delta, tmin, tmax);
        }
        (best.triangle != u32::MAX).then_some(best)
    }

    /// The first triangle a ray meets, in traversal order, within `max`
    /// (the original's boolean ray query: no nearest search).
    pub fn ray_any(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<u32> {
        let mut stack = Vec::new();
        let (near, far) = self.clip(origin, direction, 0.0, max)?;
        stack.push((0usize, near, far));
        while let Some((index, tmin, tmax)) = stack.pop() {
            let node = self.nodes[index];
            if node.is_leaf() {
                let start = node.triangle_start();
                for &tri in &self.triangles[start..start + node.triangle_count() as usize] {
                    if ray_any_triangle(self.mesh[tri as usize], origin, direction, max) {
                        return Some(tri);
                    }
                }
                continue;
            }
            visit_children(&mut stack, index, node, origin, direction, tmin, tmax);
        }
        None
    }

    /// Whether any triangle comes within `radius` of `point`, searching no
    /// farther than `max` (the projection queries' radius-expanded box
    /// traversal).
    pub fn sphere_overlap(&self, point: Vec3, radius: f32, max: f32) -> Option<u32> {
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let node = self.nodes[index];
            if node.is_leaf() {
                let start = node.triangle_start();
                for &tri in &self.triangles[start..start + node.triangle_count() as usize] {
                    if point_triangle_distance(point, self.mesh[tri as usize]) <= radius.min(max) {
                        return Some(tri);
                    }
                }
                continue;
            }
            let axis = node.axis();
            let plane = node.value;
            let left = index + node.child_offset() / 8;
            let p = point[axis];
            if p - radius <= plane {
                stack.push(left);
            }
            if p + radius >= plane {
                stack.push(left + 1);
            }
        }
        None
    }

    /// The triangle nearest to `point`, among those within `radius` of it
    /// and no farther than `max`. Returns its index and distance
    /// (the projection query of the original's picking).
    pub fn closest(&self, point: Vec3, radius: f32, max: f32) -> Option<Hit> {
        let mut best = Hit {
            triangle: u32::MAX,
            distance: max,
        };
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let node = self.nodes[index];
            if node.is_leaf() {
                let start = node.triangle_start();
                for &tri in &self.triangles[start..start + node.triangle_count() as usize] {
                    let d = point_triangle_distance(point, self.mesh[tri as usize]);
                    if d <= radius && d < best.distance {
                        best = Hit {
                            triangle: tri,
                            distance: d,
                        };
                    }
                }
                continue;
            }
            let axis = node.axis();
            let plane = node.value;
            let left = index + node.child_offset() / 8;
            if point[axis] - radius <= plane {
                stack.push(left);
            }
            if point[axis] + radius >= plane {
                stack.push(left + 1);
            }
        }
        (best.triangle != u32::MAX).then_some(best)
    }

    /// Whether a sphere of `radius` whose centre moves from `a` to `b`
    /// touches the mesh (the capsule query of the dynamics volumes).
    pub fn capsule_overlap(&self, a: Vec3, b: Vec3, radius: f32) -> Option<u32> {
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let node = self.nodes[index];
            if node.is_leaf() {
                let start = node.triangle_start();
                for &tri in &self.triangles[start..start + node.triangle_count() as usize] {
                    if segment_triangle_distance(a, b, self.mesh[tri as usize]) <= radius {
                        return Some(tri);
                    }
                }
                continue;
            }
            let axis = node.axis();
            let plane = node.value;
            let left = index + node.child_offset() / 8;
            let lo = a[axis].min(b[axis]) - radius;
            let hi = a[axis].max(b[axis]) + radius;
            if lo <= plane {
                stack.push(left);
            }
            if hi >= plane {
                stack.push(left + 1);
            }
        }
        None
    }

    /// Clips a ray to the tree's bounds. `None` when it misses, with the
    /// original's slab comparison order and its initial `tmin` of 0.
    fn clip(
        &self,
        origin: Vec3,
        direction: Vec3,
        mut tmin: f32,
        mut tmax: f32,
    ) -> Option<(f32, f32)> {
        for axis in 0..3 {
            let d = direction[axis];
            if d == 0.0 {
                if origin[axis] < self.min[axis] || origin[axis] > self.max[axis] {
                    return None;
                }
                continue;
            }
            let inverse = 1.0 / d;
            let t1 = (self.min[axis] - origin[axis]) * inverse;
            let t2 = (self.max[axis] - origin[axis]) * inverse;
            if t1 < t2 {
                if t1 > tmin {
                    tmin = t1;
                }
                if t2 < tmax {
                    tmax = t2;
                }
            } else {
                if t2 > tmin {
                    tmin = t2;
                }
                if t1 < tmax {
                    tmax = t1;
                }
            }
            if tmax < tmin {
                return None;
            }
        }
        Some((tmin, tmax))
    }
}

struct Work {
    node: u32,
    tris: Vec<u32>,
}

/// Pushes the children of an internal node in near-to-far order, giving
/// each the part of the query's interval on its side of the split.
fn visit_children(
    stack: &mut Vec<(usize, f32, f32)>,
    index: usize,
    node: Node,
    origin: Vec3,
    direction: Vec3,
    tmin: f32,
    tmax: f32,
) {
    let axis = node.axis();
    let plane = node.value;
    let left = index + node.child_offset() / 8;
    let right = left + 1;
    let d = direction[axis];
    if d == 0.0 {
        // A direction parallel to the split stays on the origin's side.
        if origin[axis] < plane {
            stack.push((left, tmin, tmax));
        } else {
            stack.push((right, tmin, tmax));
        }
        return;
    }
    let t = (plane - origin[axis]) / d;
    let (near, far) = if d >= 0.0 {
        (left, right)
    } else {
        (right, left)
    };
    if t <= tmin {
        stack.push((far, tmin, tmax));
    } else if t >= tmax {
        stack.push((near, tmin, tmax));
    } else {
        stack.push((far, t, tmax));
        stack.push((near, tmin, t));
    }
}

fn cell_bounds(tris: &[u32], mesh: &[Triangle]) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for &id in tris {
        let (bmin, bmax) = mesh[id as usize].bounds();
        min = min.min(bmin);
        max = max.max(bmax);
    }
    if min.x > max.x {
        (Vec3::ZERO, Vec3::ZERO)
    } else {
        (min, max)
    }
}

/// The original's split choice: `None` makes a leaf.
fn choose_split(
    tris: &[u32],
    mesh: &[Triangle],
    min: Vec3,
    max: Vec3,
    flat: bool,
    count: u32,
) -> Option<(usize, f32)> {
    if count > COST_TRIANGLES {
        let mut axis = 0;
        let mut extent = max[0] - min[0];
        for candidate in 0..3 {
            if flat && candidate == 1 {
                continue;
            }
            let e = max[candidate] - min[candidate];
            if e > extent {
                extent = e;
                axis = candidate;
            }
        }
        let plane = plane_at(min, max, axis, 8);
        return Some((axis, plane));
    }
    let mut best = BEST_COST;
    let mut best_axis = usize::MAX;
    let mut best_step = 0;
    for axis in 0..3 {
        if flat && axis == 1 {
            continue;
        }
        let extent = max[axis] - min[axis];
        // The original compares the f32 extent widened to f64: below
        // 0.0001 the axis is skipped (NaN, however, is not).
        if f64::from(extent) < 0.0001 && !extent.is_nan() {
            continue;
        }
        let (ea, eb, ec) = (
            extent,
            max[(axis + 1) % 3] - min[(axis + 1) % 3],
            max[(axis + 2) % 3] - min[(axis + 2) % 3],
        );
        let denom = ea * (eb + ec) + eb * ec;
        let inverse = 1.0 / denom;
        for step in 1..=SPLIT_STEPS {
            let s = plane_at(min, max, axis, step);
            let mut left = 0u32;
            let mut right = 0u32;
            for &id in tris {
                let (bmin, bmax) = mesh[id as usize].bounds();
                if s < bmin[axis] {
                    right += 1;
                } else {
                    left += 1;
                    if s >= bmax[axis] {
                        right += 1;
                    }
                }
            }
            let r = (((s - min[axis]) * (eb + ec)) + eb * ec) * inverse;
            let l = (((max[axis] - s) * (eb + ec)) + eb * ec) * inverse;
            let cost = (right as f32) * r + (left as f32) * l + 1.0;
            if cost < best {
                best = cost;
                best_axis = axis;
                best_step = step;
            }
        }
    }
    if best_axis == usize::MAX || best <= count as f32 {
        return None;
    }
    Some((best_axis, plane_at(min, max, best_axis, best_step)))
}

/// A candidate plane at `step`/16 of the axis' extent, formed in double
/// precision as the original forms it.
fn plane_at(min: Vec3, max: Vec3, axis: usize, step: u32) -> f32 {
    let lo = f64::from(min[axis]);
    let hi = f64::from(max[axis]);
    (lo + f64::from(step) * 0.0625 * (hi - lo)) as f32
}

/// The original's nearest-hit triangle test: front faces only.
fn ray_triangle(tri: Triangle, origin: Vec3, direction: Vec3, best: &mut Hit) -> Option<f32> {
    let [v0, v1, v2] = tri.vertices;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let p = direction.cross(e2);
    let det = e1.dot(p);
    if det < EPSILON {
        return None;
    }
    let t = origin - v0;
    let u = t.dot(p);
    if u < 0.0 || u > det {
        return None;
    }
    let q = t.cross(e1);
    let v = direction.dot(q);
    if v < 0.0 || u + v > det {
        return None;
    }
    let distance = e2.dot(q) / det;
    if distance > 0.0 && distance < best.distance {
        Some(distance)
    } else {
        None
    }
}

/// The original's segment test: both faces, the far end included.
fn segment_triangle(tri: Triangle, origin: Vec3, delta: Vec3, best: &mut Hit) -> Option<f32> {
    let [v0, v1, v2] = tri.vertices;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let p = delta.cross(e2);
    let det = e1.dot(p);
    // The original clears the sign bit of the determinant before the
    // epsilon test.
    if det.abs() < EPSILON {
        return None;
    }
    let inverse = 1.0 / det;
    let t = origin - v0;
    let u = t.dot(p) * inverse;
    if u < 0.0 || u > 1.0 {
        return None;
    }
    let q = t.cross(e1);
    let v = delta.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = e2.dot(q) * inverse;
    if distance > 0.0 && distance <= best.distance {
        Some(distance)
    } else {
        None
    }
}

/// The original's boolean ray test. The projected point-in-triangle test
/// uses the two axes of the triangle's largest normal component.
fn ray_any_triangle(tri: Triangle, origin: Vec3, direction: Vec3, max: f32) -> bool {
    let (normal, offset) = tri.plane();
    // The mesh's triangle normals face the ray the original's boolean
    // query accepts (the opposite of the winding's geometric normal).
    let (normal, offset) = (-normal, -offset);
    let det = direction.dot(normal);
    if det < EPSILON {
        return false;
    }
    let distance = (offset - origin.dot(normal)) / det;
    if distance < 0.0 || distance > max {
        return false;
    }
    point_in_triangle(origin + direction * distance, tri)
}

/// Whether a point projected onto the triangle's dominant plane lies
/// inside it.
fn point_in_triangle(point: Vec3, tri: Triangle) -> bool {
    let n = tri.normal();
    let abs = n.abs();
    let (a, b) = if abs.x >= abs.y && abs.x >= abs.z {
        (1, 2)
    } else if abs.y >= abs.z {
        (0, 2)
    } else {
        (0, 1)
    };
    let [v0, v1, v2] = tri.vertices;
    let p = Vec3::new(point[a], point[b], 0.0);
    let a0 = Vec3::new(v0[a], v0[b], 0.0);
    let a1 = Vec3::new(v1[a], v1[b], 0.0);
    let a2 = Vec3::new(v2[a], v2[b], 0.0);
    let cross = |u: Vec3, v: Vec3| u.x * v.y - u.y * v.x;
    let c0 = cross(a1 - a0, p - a0);
    let c1 = cross(a2 - a1, p - a1);
    let c2 = cross(a0 - a2, p - a2);
    (c0 >= 0.0 && c1 >= 0.0 && c2 >= 0.0) || (c0 <= 0.0 && c1 <= 0.0 && c2 <= 0.0)
}

/// The distance from a point to a triangle (0 inside it).
pub fn point_triangle_distance(point: Vec3, tri: Triangle) -> f32 {
    let [a, b, c] = tri.vertices;
    // Closest point via barycentric region tests.
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (point - a).length();
    }
    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (point - b).length();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (point - (a + ab * v)).length();
    }
    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (point - c).length();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (point - (a + ac * w)).length();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (point - (b + (c - b) * w)).length();
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (point - (a + ab * v + ac * w)).length()
}

/// The distance between a segment and a triangle: the smallest of the
/// segment-to-edge distances, the segment's crossings and the projected
/// closest point.
pub fn segment_triangle_distance(a: Vec3, b: Vec3, tri: Triangle) -> f32 {
    if segment_plane_inside(a, b, tri).is_some() {
        return 0.0;
    }
    let mut distance = f32::INFINITY;
    for i in 0..3 {
        let p = tri.vertices[i];
        let q = tri.vertices[(i + 1) % 3];
        distance = distance.min(segment_segment_distance(a, b, p, q));
    }
    for p in tri.vertices {
        distance = distance.min(point_segment_distance(p, a, b));
    }
    let _ = point_triangle_distance(a, tri);
    distance
}

fn segment_plane_inside(a: Vec3, b: Vec3, tri: Triangle) -> Option<f32> {
    let (normal, offset) = tri.plane();
    let denom = normal.dot(b - a);
    if denom == 0.0 {
        return None;
    }
    let t = (offset - normal.dot(a)) / denom;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    point_in_triangle(a + (b - a) * t, tri).then_some(t)
}

fn point_segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len = ab.length_squared();
    if len == 0.0 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn segment_segment_distance(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.length_squared();
    let e = d2.length_squared();
    let f = d2.dot(r);
    let (mut s, mut t);
    if a <= EPSILON && e <= EPSILON {
        return (p1 - p2).length();
    }
    if a <= EPSILON {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= EPSILON {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            s = if denom != 0.0 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
        }
    }
    (p1 + d1 * s - (p2 + d2 * t)).length()
}

/// A terrain block's tree (`TTErrainkdTree`): the cell grid of one 6 m
/// block, 30 × 30 cells of 20 cm, split in halves until a node covers at
/// most 3 × 3 cells. Every leaf lists two triangles per cell.
///
/// The tree is built once and shared by every block, translated by the
/// block's offset (the original's split planes move, the cell ranges and
/// triangles do not).
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainKdTree {
    pub nodes: Vec<Node>,
    pub triangles: Vec<u32>,
    /// Cells along a side (30 for a 6 m block of 20 cm cells).
    pub cells: usize,
    /// The cell size (0.2 m).
    pub spacing: f32,
    /// The block's cached bounds, recomputed by [`Self::attach`].
    pub min: Vec3,
    pub max: Vec3,
}

/// Cells along a terrain block's side.
pub const TERRAIN_CELLS: usize = 30;
/// A terrain block's cell size, metres.
pub const TERRAIN_SPACING: f32 = 0.2;
/// A terrain leaf covers at most this many cells along a side.
pub const TERRAIN_LEAF_CELLS: u8 = 3;

impl Default for TerrainKdTree {
    fn default() -> Self {
        Self::build(TERRAIN_CELLS, TERRAIN_SPACING)
    }
}

impl TerrainKdTree {
    /// Builds the block tree over `cells` × `cells` cells of `spacing`
    /// metres. A node's four cell bounds are bytes packed in the second
    /// word: min x, max x, min z, max z.
    pub fn build(cells: usize, spacing: f32) -> Self {
        let half = cells as f32 * spacing * 0.5;
        let mut tree = Self {
            nodes: Vec::new(),
            triangles: Vec::new(),
            cells,
            spacing,
            min: Vec3::splat(-half),
            max: Vec3::splat(half),
        };
        if cells == 0 {
            return tree;
        }
        let n = cells as u8;
        tree.nodes.push(Node {
            word: LEAF,
            value: f32::from_bits(u32::from_le_bytes([0, n, 0, n])),
        });
        let mut stack = vec![0u32];
        while let Some(index) = stack.pop() {
            let range = tree.range(index);
            let dx = range[1] - range[0];
            let dz = range[3] - range[2];
            let axis = if dz > dx { 2usize } else { 0 };
            let first = tree.nodes.len() as u32;
            tree.nodes.push(tree.nodes[index as usize]);
            tree.nodes.push(tree.nodes[index as usize]);
            let mid = (range[axis] + range[axis + 1]) / 2;
            // The left child keeps the parent's lower bound and ends at the
            // middle; the right child begins at the middle.
            tree.set_range(first, range_with(range, axis + 1, mid));
            tree.set_range(first + 1, range_with(range, axis, mid));
            let offset = (first as usize - index as usize) * 8;
            tree.nodes[index as usize] = Node {
                word: offset as u32 | axis as u32,
                value: (mid as f32 - cells as f32 * 0.5) * spacing,
            };
            for child in [first, first + 1] {
                let r = tree.range(child);
                let cdx = r[1] - r[0];
                let cdz = r[3] - r[2];
                if cdx > TERRAIN_LEAF_CELLS || cdz > TERRAIN_LEAF_CELLS {
                    stack.push(child);
                } else {
                    tree.make_leaf(child);
                }
            }
        }
        tree
    }

    fn range(&self, node: u32) -> [u8; 4] {
        self.nodes[node as usize].value.to_bits().to_le_bytes()
    }

    fn set_range(&mut self, node: u32, range: [u8; 4]) {
        self.nodes[node as usize].value = f32::from_bits(u32::from_le_bytes(range));
    }

    fn make_leaf(&mut self, node: u32) {
        let r = self.range(node);
        let start = self.triangles.len() as u32;
        let mut count = 0u32;
        for j in r[2]..r[3] {
            for i in r[0]..r[1] {
                let cell = (i as u32 + self.cells as u32 * j as u32) * 2;
                self.triangles.push(cell);
                self.triangles.push(cell + 1);
                count += 2;
            }
        }
        self.nodes[node as usize] = Node {
            word: count | LEAF,
            value: start as f32,
        };
    }

    /// Translates every internal node's split plane by `offset`: the
    /// original's shared-tree clone for a block. The cached bounds move
    /// with the block.
    pub fn translate(&mut self, offset: Vec3) {
        for node in &mut self.nodes {
            if !node.is_leaf() {
                node.value += offset[node.axis()];
            }
        }
        self.min += offset;
        self.max += offset;
    }

    /// A translated copy (the original shares the arrays and moves the
    /// planes on the copy).
    pub fn translated(&self, offset: Vec3) -> Self {
        let mut tree = self.clone();
        tree.translate(offset);
        tree
    }

    /// Caches the bounds of the mesh the tree was attached to (the
    /// original's attach slot). The cell ranges and triangles do not move.
    pub fn attach(&mut self, min: Vec3, max: Vec3) {
        self.min = min;
        self.max = max;
    }

    /// The triangles of a cell: two per cell, index `(i + n·j)·2`.
    pub fn cell_triangles(cell: u32) -> [u32; 2] {
        [cell * 2, cell * 2 + 1]
    }
}

fn range_with(range: [u8; 4], at: usize, value: u8) -> [u8; 4] {
    let mut r = range;
    r[at] = value;
    r
}

/// The cell of a world point in a terrain block (`(x/z) / 0.2` rounded the
/// original's way, floor).
pub fn terrain_cell(point: Vec3, spacing: f32) -> (u8, u8) {
    let x = (point.x / spacing).floor().clamp(0.0, 255.0) as u8;
    let z = (point.z / spacing).floor().clamp(0.0, 255.0) as u8;
    (x, z)
}

#[cfg(test)]
#[path = "kd_tree_test.rs"]
mod tests;
