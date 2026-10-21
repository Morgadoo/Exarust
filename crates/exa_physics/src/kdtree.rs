//! The background mesh kd-tree plumbing: the two locked ring queues the
//! builder thread drains, the frame hook that installs finished trees, and
//! the ray, sphere and capsule queries a mesh and its instances receive.
//!
//! The original's `TKDtreeBuilderThread` is an idle-priority thread. Meshes
//! wait in a grow-on-full ring (initial 256 entries, grown by 128); the
//! thread pops one, skips a mesh flagged deleted, builds a `TkdTree` for it
//! and, when the queue asks, re-queues it. The frame hook drains a second
//! ring of finished meshes and installs the query callbacks (ray, sphere,
//! capsule …) into the mesh, then copies them to every instance. A stop
//! flag ends the thread's loop, and the session-end reset empties both
//! rings.
//!
//! Exarust's collider builder in `exa_physics` already copies meshes in
//! budgeted batches and builds on the async compute pool; this module
//! supplies the ring semantics it uses for its queues and the query
//! install a mesh and its instances share.

use bevy::math::Vec3;

/// A grow-on-full FIFO ring: the original pushes into a fixed array, grows
/// it by 128 slots when full, and pops from the head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ring<T> {
    slots: Vec<Option<T>>,
    head: usize,
    count: usize,
    /// The queue's re-queue flag: the builder puts a finished mesh back.
    pub requeue: bool,
}

impl<T> Ring<T> {
    /// A ring with the original's initial capacity.
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: std::iter::repeat_with(|| None)
                .take(capacity.max(1))
                .collect(),
            head: 0,
            count: 0,
            requeue: false,
        }
    }

    /// Pushes to the tail, growing by 128 slots when full.
    pub fn push(&mut self, value: T) {
        if self.count == self.slots.len() {
            self.grow(128);
        }
        let at = (self.head + self.count) % self.slots.len();
        self.slots[at] = Some(value);
        self.count += 1;
    }

    fn grow(&mut self, by: usize) {
        let mut old = std::mem::take(&mut self.slots);
        let old_len = old.len();
        let mut slots: Vec<Option<T>> =
            std::iter::repeat_with(|| None).take(old_len + by).collect();
        for i in 0..self.count {
            slots[i] = old[(self.head + i) % old_len].take();
        }
        self.slots = slots;
        self.head = 0;
    }

    /// Pops the head.
    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        let value = self.slots[self.head].take();
        self.head = (self.head + 1) % self.slots.len();
        self.count -= 1;
        value
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Empties the ring (the session-end reset).
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
        self.head = 0;
        self.count = 0;
    }
}

impl<T> Default for Ring<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The builder thread's state: the idle priority, the stop flag and the
/// running flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuilderThread {
    pub idle_priority: bool,
    pub running: bool,
    pub stop: bool,
}

impl Default for BuilderThread {
    fn default() -> Self {
        Self {
            idle_priority: true,
            running: false,
            stop: false,
        }
    }
}

impl BuilderThread {
    /// Starts the thread loop (the original lowers its priority and calls
    /// the build loop); a stopped thread is restarted.
    pub fn start(&mut self) -> bool {
        if self.running {
            return false;
        }
        self.running = true;
        self.stop = false;
        true
    }

    /// The stop flag: the loop returns after the mesh it is on.
    pub fn stop(&mut self) {
        self.stop = true;
    }

    /// Whether the loop runs another mesh.
    pub fn next(&self) -> bool {
        self.running && !self.stop
    }

    /// The loop has returned.
    pub fn finished(&mut self) {
        self.running = false;
    }

    /// The session-end reset: both rings empty and the thread stops.
    pub fn reset(&mut self) {
        self.stop = true;
        self.running = false;
    }
}

/// The query callbacks installed on a mesh (the original's function
/// pointers at `+0x3a0` … `+0x418`). The tree answers them; the flags only
/// say which a mesh and its instances answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KdQueries {
    pub ray: bool,
    pub sphere: bool,
    pub capsule: bool,
}

impl KdQueries {
    /// All three installed (the original's install sets every one at once).
    pub const ALL: Self = Self {
        ray: true,
        sphere: true,
        capsule: true,
    };
}

/// The per-mesh install: the queries and the instances they are copied to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryInstall {
    pub queries: KdQueries,
    pub instances: Vec<KdQueries>,
}

impl QueryInstall {
    /// Installs the queries into the mesh and copies them to every
    /// instance (the original's install loop).
    pub fn install(&mut self, queries: KdQueries) {
        self.queries = queries;
        self.instances.fill(queries);
    }

    /// Registers an instance (a mesh sharing this tree) with no queries
    /// until the install runs.
    pub fn add_instance(&mut self) {
        self.instances.push(KdQueries::default());
    }
}

/// A triangle in a mesh's tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub points: [Vec3; 3],
}

impl Triangle {
    /// The closest point on the triangle to `p`, as Ericson's barycentric
    /// region walk (the engine's per-face routines are outside the group;
    /// this is Exarust's own triangle test).
    pub fn closest_point(&self, p: Vec3) -> Vec3 {
        let [a, b, c] = self.points;
        let ab = b - a;
        let ac = c - a;
        let ap = p - a;
        let d1 = ab.dot(ap);
        let d2 = ac.dot(ap);
        if d1 <= 0.0 && d2 <= 0.0 {
            return a;
        }
        let bp = p - b;
        let d3 = ab.dot(bp);
        let d4 = ac.dot(bp);
        if d3 >= 0.0 && d4 <= d3 {
            return b;
        }
        let vc = d1 * d4 - d3 * d2;
        if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            let v = d1 / (d1 - d3);
            return a + ab * v;
        }
        let cp = p - c;
        let d5 = ab.dot(cp);
        let d6 = ac.dot(cp);
        if d6 >= 0.0 && d5 <= d6 {
            return c;
        }
        let vb = d5 * d2 - d1 * d6;
        if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            let w = d2 / (d2 - d6);
            return a + ac * w;
        }
        let va = d3 * d6 - d5 * d4;
        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
            return b + (c - b) * w;
        }
        let denom = 1.0 / (va + vb + vc);
        let v = vb * denom;
        let w = vc * denom;
        a + ab * v + ac * w
    }

    /// The squared distance from `p` to the triangle.
    pub fn distance_squared(&self, p: Vec3) -> f32 {
        self.closest_point(p).distance_squared(p)
    }

    /// Whether the segment `from`..`to` meets the triangle, by Möller and
    /// Trumbore's test (two-sided).
    pub fn segment_hit(&self, from: Vec3, to: Vec3) -> bool {
        let direction = to - from;
        let [a, b, c] = self.points;
        let e1 = b - a;
        let e2 = c - a;
        let p = direction.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-12 {
            return false;
        }
        let inv = 1.0 / det;
        let t = from - a;
        let u = t.dot(p) * inv;
        if !(0.0..=1.0).contains(&u) {
            return false;
        }
        let q = t.cross(e1);
        let v = direction.dot(q) * inv;
        if v < 0.0 || u + v > 1.0 {
            return false;
        }
        let along = e2.dot(q) * inv;
        (0.0..=1.0).contains(&along)
    }
}

/// The nearest distance between two segments.
fn segment_segment_distance_squared(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    let s;
    let t;
    if a <= 1e-12 && e <= 1e-12 {
        return r.length_squared();
    }
    if a <= 1e-12 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-12 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let s0 = if denom.abs() > 1e-12 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            } else {
                t = t0;
                s = s0;
            }
        }
    }
    (p1 + d1 * s).distance_squared(p2 + d2 * t)
}

/// A mesh's triangle set and its three queries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KdTree {
    pub triangles: Vec<Triangle>,
}

impl KdTree {
    /// The original's ray query: the nearest triangle the ray meets within
    /// `max`, as (distance, triangle index).
    pub fn ray(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<(f32, usize)> {
        let end = origin + direction.normalize_or_zero() * max;
        let mut best: Option<(f32, usize)> = None;
        for (i, triangle) in self.triangles.iter().enumerate() {
            if !triangle.segment_hit(origin, end) {
                continue;
            }
            // The hit's distance along the ray, from the closest point on
            // the triangle in the direction's plane.
            let t = (triangle.closest_point(origin) - origin)
                .dot(direction.normalize_or_zero())
                .max(0.0);
            if best.is_none_or(|(b, _)| t < b) {
                best = Some((t, i));
            }
        }
        best
    }

    /// The first triangle within `radius` of `centre`.
    pub fn sphere(&self, centre: Vec3, radius: f32) -> Option<usize> {
        self.triangles
            .iter()
            .position(|t| t.distance_squared(centre) <= radius * radius)
    }

    /// The first triangle within `radius` of the segment `from`..`to`.
    pub fn capsule(&self, from: Vec3, to: Vec3, radius: f32) -> Option<usize> {
        let r2 = radius * radius;
        self.triangles.iter().position(|t| {
            let [a, b, c] = t.points;
            let mut best = segment_segment_distance_squared(from, to, a, b);
            best = best.min(segment_segment_distance_squared(from, to, b, c));
            best = best.min(segment_segment_distance_squared(from, to, c, a));
            best = best.min(t.distance_squared(from));
            best = best.min(t.distance_squared(to));
            best <= r2
        })
    }
}

/// The three bits of a face's flag word, sign-extended: bits 0–2, negated
/// when bit 3 is set.
pub fn decode_three_bits(word: u16) -> i16 {
    let value = (word & 7) as i16;
    if word & 8 != 0 { -value } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_grows_by_128_and_pops_in_order() {
        let mut ring: Ring<u32> = Ring::with_capacity(4);
        for i in 0..4 {
            ring.push(i);
        }
        assert_eq!(ring.len(), 4);
        assert_eq!(ring.capacity(), 4);
        ring.push(4);
        assert_eq!(ring.capacity(), 4 + 128);
        assert_eq!(ring.len(), 5);
        for i in 0..5 {
            assert_eq!(ring.pop(), Some(i));
        }
        assert!(ring.pop().is_none());
        ring.clear();
        assert!(ring.is_empty());
        // A default ring starts with 256 slots.
        assert_eq!(Ring::<u8>::new().capacity(), 256);
    }

    #[test]
    fn the_builder_thread_stops_and_resets() {
        let mut thread = BuilderThread::default();
        assert!(thread.idle_priority);
        assert!(!thread.next());
        assert!(thread.start());
        assert!(thread.next());
        assert!(!thread.start());
        thread.stop();
        assert!(!thread.next());
        thread.finished();
        assert!(!thread.running);
        thread.reset();
        assert!(!thread.next());
        assert!(thread.start());
    }

    #[test]
    fn installing_queries_copies_them_to_instances() {
        let mut install = QueryInstall::default();
        install.add_instance();
        install.add_instance();
        assert_eq!(install.instances, [KdQueries::default(); 2]);
        install.install(KdQueries::ALL);
        assert_eq!(install.queries, KdQueries::ALL);
        assert!(install.instances.iter().all(|q| *q == KdQueries::ALL));
    }

    #[test]
    fn the_three_bits_are_sign_extended() {
        assert_eq!(decode_three_bits(0), 0);
        assert_eq!(decode_three_bits(5), 5);
        assert_eq!(decode_three_bits(0x8), 0);
        assert_eq!(decode_three_bits(0x8 | 3), -3);
        assert_eq!(decode_three_bits(0x8 | 7), -7);
    }

    fn tri() -> KdTree {
        KdTree {
            triangles: vec![Triangle {
                points: [Vec3::ZERO, Vec3::X, Vec3::Z],
            }],
        }
    }

    #[test]
    fn the_triangle_queries_find_their_hits() {
        let tree = tri();
        assert_eq!(
            tree.ray(Vec3::new(0.25, 1.0, 0.25), -Vec3::Y, 5.0)
                .map(|h| h.1),
            Some(0)
        );
        assert!(tree.ray(Vec3::new(2.0, 1.0, 2.0), -Vec3::Y, 5.0).is_none());
        assert_eq!(tree.sphere(Vec3::new(0.25, 1.0, 0.25), 1.1), Some(0));
        assert_eq!(tree.sphere(Vec3::new(0.25, 1.0, 0.25), 0.9), None);
        assert_eq!(
            tree.capsule(Vec3::new(0.25, 1.0, 0.25), Vec3::new(0.25, 2.0, 0.25), 1.1),
            Some(0)
        );
        assert_eq!(
            tree.capsule(Vec3::new(0.25, 1.0, 0.25), Vec3::new(0.25, 2.0, 0.25), 0.9),
            None
        );
    }

    #[test]
    fn the_triangle_closest_point_matches_its_regions() {
        let t = Triangle {
            points: [Vec3::ZERO, Vec3::X, Vec3::Z],
        };
        let close = |p| t.closest_point(p);
        assert_eq!(close(Vec3::new(-1.0, 0.0, -1.0)), Vec3::ZERO);
        assert_eq!(close(Vec3::new(2.0, 0.0, -1.0)), Vec3::X);
        assert_eq!(
            close(Vec3::new(0.25, 1.0, 0.25)),
            Vec3::new(0.25, 0.0, 0.25)
        );
        assert!(t.distance_squared(Vec3::new(0.25, 1.0, 0.25)) - 1.0 < 1e-6);
    }
}
