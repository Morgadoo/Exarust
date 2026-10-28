//! Body parts against the level, as the original keeps them out of it: each
//! sphere collider is tested against the front of the level's triangles,
//! where it is (static) or along its path from where the last tick left it
//! (swept). A hit pushes the part at once and is kept as a contact, a plane
//! the part's point must stay above through the next tick's iterations,
//! with friction from the part's own coefficient.
//! Evidence: `docs/formats/RSQ.md` (Physical body).

use bevy::prelude::*;

/// A stored contact's plane lies this far (m) under the touched surface.
pub(crate) const CONTACT_SLOP: f32 = 0.001;
/// A centre this close (squared, m²) to a triangle lies on it.
const ON_TRIANGLE2: f32 = 1e-12;

/// A contact with the level: a point of a part (weights over its particles)
/// kept on the outer side of a plane.
#[derive(Debug, Clone, Copy)]
pub struct LevelContact {
    pub weights: [f32; 4],
    /// The touched point on the triangle.
    pub point: Vec3,
    pub normal: Vec3,
    /// The plane `normal · x = plane`, [`CONTACT_SLOP`] under the point.
    pub plane: f32,
}

/// Static level geometry: triangles whose front is the side
/// `(b − a) × (c − a)` points to. Only the front pushes.
pub trait Level {
    /// Pieces of geometry (meshes) whose bounds meet the box `lo..hi`.
    fn pieces(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>);
    /// Triangles of `piece` meeting the box `lo..hi`, in world space.
    fn triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<[Vec3; 3]>);

    /// The original's level mask of a piece (collider): 1, or 1|4 for a
    /// flagged or thin horizontal mesh. A body whose own mask shares a bit
    /// skips the piece. Exarust's colliders do not carry their masks yet
    /// (the level loader does not hand them over), so every piece is the
    /// plain 1; a test or a future loader can override it.
    fn mask(&self, _piece: u32) -> u32 {
        crate::sector::MASK_SMALL
    }

    /// Whether `piece` is the stand-in of a rigid body's meshes, moved with
    /// it: the character capsules meet it, bodies skip it (they meet the
    /// rigid body through their pairs). None by default.
    fn movable(&self, _piece: u32) -> bool {
        false
    }

    /// The original's Height Query: a sphere of `radius` whose lowest point
    /// starts at `at` is swept straight down by at most `depth`. Returns
    /// the height of its lowest point where it first meets the front of an
    /// upward face, or `None`. A sphere that starts against a face finds it
    /// at `at` (the original counts a negative distance as 0). Only the
    /// level answers it: living bodies are not part of it.
    fn height_below(&self, at: Vec3, radius: f32, depth: f32) -> Option<f32> {
        height_query(self, at, radius, depth)
    }

    /// Whether `piece` takes part in the original's Ground Query. The
    /// original meets only meshes and bodies whose node carries the floor
    /// bit (0x800000 in its second flag word: 85 library pieces and 2
    /// objects ship with it), and skips a disabled body. Exarust's level
    /// colliders do not carry that bit yet, so by default no piece is
    /// floor and the query misses, as it does in the original over every
    /// unflagged mesh.
    fn floor_piece(&self, _piece: u32) -> bool {
        false
    }

    /// The original's Ground Query: a point ray from `at` straight down at
    /// most `depth`, against the front of upward faces of floor pieces
    /// only. Returns the height of the first face it meets, or `None`.
    /// The feet routine uses it from 30 cm over the ground to find the sole
    /// floor under the toe (60 cm down).
    fn floor_below(&self, at: Vec3, depth: f32) -> Option<f32> {
        if !(depth > 0.0 && at.is_finite()) {
            return None;
        }
        floor_query(self, at, depth)
    }
}

/// The Ground Query over the level's floor pieces: the highest upward face
/// under the point. The ray is vertical, so a face's hit is where its
/// plane sits over the point, if the point is over the triangle and at or
/// above the plane.
fn floor_query<L: Level + ?Sized>(level: &L, at: Vec3, depth: f32) -> Option<f32> {
    let (lo, hi) = (at - Vec3::Y * depth, at);
    let (mut pieces, mut triangles) = (Vec::new(), Vec::new());
    level.pieces(lo, hi, &mut pieces);
    for &piece in &pieces {
        if !level.floor_piece(piece) {
            continue;
        }
        level.triangles(piece, lo, hi, &mut triangles);
    }
    let mut nearest: Option<f32> = None;
    for t in &triangles {
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.y <= 0.0 || n.dot(at - t[0]) < 0.0 {
            continue;
        }
        // The plane's height over the point, if the point is over the
        // triangle. Only the highest such face is the floor.
        let y = (n.dot(t[0]) - n.x * at.x - n.z * at.z) / n.y;
        if y > at.y || at.y - y > depth {
            continue;
        }
        if !inside(t, n, Vec3::new(at.x, y, at.z)) {
            continue;
        }
        if nearest.is_none_or(|best| y > best) {
            nearest = Some(y);
        }
    }
    nearest
}

/// How deep (m) a Height Query looks first; it looks the whole way down
/// only when nothing is that near (the answer is the same, the column of
/// triangles to test much shorter).
const NEAR_DEPTH: f32 = 2.0;

/// The Height Query over a level's triangles: the near column first, then
/// the whole one.
fn height_query<L: Level + ?Sized>(level: &L, at: Vec3, radius: f32, depth: f32) -> Option<f32> {
    if !(radius > 0.0 && depth > 0.0 && at.is_finite()) {
        return None;
    }
    let near = depth.min(NEAR_DEPTH);
    sweep_down(level, at, radius, near).or_else(|| {
        (depth > near)
            .then(|| sweep_down(level, at, radius, depth))
            .flatten()
    })
}

/// The sphere swept down `depth` against the triangles of the column under
/// it, highest first, until none can be met sooner.
fn sweep_down<L: Level + ?Sized>(level: &L, at: Vec3, radius: f32, depth: f32) -> Option<f32> {
    let centre = at + Vec3::Y * radius;
    let side = Vec3::new(radius, 0.0, radius);
    let (lo, hi) = (
        at - side - Vec3::Y * depth,
        centre + side + Vec3::Y * radius,
    );
    let (mut pieces, mut triangles) = (Vec::new(), Vec::new());
    level.pieces(lo, hi, &mut pieces);
    for &piece in &pieces {
        level.triangles(piece, lo, hi, &mut triangles);
    }
    // Upward faces with the sphere in front of them, highest first.
    let mut faces: Vec<(f32, [Vec3; 3])> = triangles
        .iter()
        .filter(|t| {
            let n = (t[1] - t[0]).cross(t[2] - t[0]);
            n.y > 0.0 && n.dot(centre - t[0]) >= 0.0
        })
        .map(|t| (t[0].y.max(t[1].y).max(t[2].y), *t))
        .collect();
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut nearest: Option<f32> = None;
    for (top, t) in faces {
        let reach = nearest.unwrap_or(depth);
        // A face stops the sphere's lowest point at its own highest point
        // or under it: one no higher than where it already stops cannot
        // stop it sooner.
        if top <= centre.y - reach - radius {
            break;
        }
        if let Some(d) = drop_onto(&t, centre, radius, reach) {
            nearest = Some(d);
        }
    }
    nearest.map(|d| at.y - d)
}

/// How far (at most `reach`) a sphere of radius `r` centred at `c` drops
/// straight down before it meets triangle `t`: the first of its face, its
/// edges and its corners it touches; 0 when it starts against it.
pub(crate) fn drop_onto(t: &[Vec3; 3], c: Vec3, r: f32, reach: f32) -> Option<f32> {
    let n = (t[1] - t[0]).cross(t[2] - t[0]).try_normalize()?;
    let r2 = r * r;
    let mut first = f32::INFINITY;
    // The face: the centre one radius over the plane (or nearer to start
    // with), over the triangle.
    let s = n.dot(c - t[0]);
    if n.y > 0.0 && s >= 0.0 {
        let d = ((s - r) / n.y).max(0.0);
        let below = c - Vec3::Y * d;
        if inside(t, n, below - n * n.dot(below - t[0])) {
            first = d;
        }
    }
    for k in 0..3 {
        let (p, q) = (t[k], t[(k + 1) % 3]);
        // A corner: the centre one radius from it.
        let w = c - p;
        let w2 = w.length_squared();
        if w2 <= r2 {
            return Some(0.0);
        }
        let disc = w.y * w.y - (w2 - r2);
        if disc >= 0.0 && w.y > 0.0 {
            first = first.min(w.y - disc.sqrt());
        }
        // An edge: the centre one radius from its line, between its ends.
        let length = (q - p).length();
        if length <= 0.0 {
            continue;
        }
        let u = (q - p) / length;
        let a = 1.0 - u.y * u.y;
        if a <= 1e-9 {
            continue;
        }
        let along = w.dot(u);
        let b = -w.y + along * u.y;
        let rest = w2 - along * along - r2;
        let disc = b * b - a * rest;
        if disc < 0.0 {
            continue;
        }
        let d = ((-b - disc.sqrt()) / a).max(0.0);
        let at = along + d * -u.y;
        if (0.0..=length).contains(&at) && (rest > 0.0 || d == 0.0) && (rest <= 0.0 || -b > 0.0) {
            first = first.min(d);
        }
    }
    (first <= reach).then_some(first)
}

/// Whether `x`, on the plane of triangle `t` with normal `n`, lies inside it.
fn inside(t: &[Vec3; 3], n: Vec3, x: Vec3) -> bool {
    (0..3).all(|k| {
        let (p, q) = (t[k], t[(k + 1) % 3]);
        (q - p).cross(x - p).dot(n) >= -1e-7
    })
}

/// A plain triangle list, one piece.
impl Level for [[Vec3; 3]] {
    fn pieces(&self, _: Vec3, _: Vec3, out: &mut Vec<u32>) {
        if !self.is_empty() {
            out.push(0);
        }
    }

    fn triangles(&self, _: u32, lo: Vec3, hi: Vec3, out: &mut Vec<[Vec3; 3]>) {
        out.extend(self.iter().filter(|t| {
            let (a, b) = (t[0].min(t[1]).min(t[2]), t[0].max(t[1]).max(t[2]));
            a.cmple(hi).all() && b.cmpge(lo).all()
        }));
    }

    /// A synthetic triangle list stands for a floor-flagged mesh.
    fn floor_piece(&self, _: u32) -> bool {
        true
    }
}

/// The level colliders of the physics world: every fixed triangle mesh.
impl Level for rapier3d::prelude::PhysicsWorld {
    fn pieces(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) {
        use rapier3d::prelude::*;
        out.extend(
            self.intersect_aabb_conservative(Aabb::new(lo, hi), QueryFilter::only_fixed())
                .filter(|(h, c)| c.shape().as_trimesh().is_some() && crate::solid(*h, c))
                .map(|(h, _)| h.into_raw_parts().0),
        );
    }

    fn movable(&self, piece: u32) -> bool {
        self.colliders
            .get_unknown_gen(piece)
            .is_some_and(|(c, _)| c.user_data == crate::prop::STAND_IN)
    }

    fn triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<[Vec3; 3]>) {
        use rapier3d::prelude::*;
        let Some((collider, _)) = self.colliders.get_unknown_gen(piece) else {
            return;
        };
        let Some(mesh) = collider.shape().as_trimesh() else {
            return;
        };
        let pose = collider.position();
        let local = Aabb::new(lo, hi).transform_by(&pose.inverse());
        for i in mesh.bvh().intersect_aabb(&local) {
            let t = mesh.triangle(i);
            out.push([t.a, t.b, t.c].map(|v| pose.transform_point(v)));
        }
    }
}

/// The closest point on triangle `t` to `x`.
fn closest_point(t: &[Vec3; 3], x: Vec3) -> Vec3 {
    use rapier3d::parry::query::PointQuery;
    rapier3d::parry::shape::Triangle::new(t[0], t[1], t[2])
        .project_local_point(x, false)
        .point
}

/// The sphere `c`, `r` against the front of triangle `t`: the closest point
/// on it and the push off it (along the closest-point direction).
pub(crate) fn sphere_hit(t: &[Vec3; 3], c: Vec3, r: f32) -> Option<(Vec3, Vec3)> {
    let n = (t[1] - t[0]).cross(t[2] - t[0]).try_normalize()?;
    let s = n.dot(c - t[0]);
    if !(0.0..r).contains(&s) {
        return None;
    }
    let q = closest_point(t, c);
    let v = c - q;
    let d2 = v.length_squared();
    if d2 >= r * r {
        return None;
    }
    if d2 < ON_TRIANGLE2 {
        return Some((q, n * r));
    }
    let d = d2.sqrt();
    Some((q, v * ((r - d) / d)))
}

/// The sphere of radius `r` moving from `start` to `end` against the front
/// of triangle `t`. While the end stays in front this is the static test at
/// the end; an end that crossed the plane is taken back to where the sphere
/// first touched.
pub(crate) fn swept_hit(t: &[Vec3; 3], start: Vec3, end: Vec3, r: f32) -> Option<(Vec3, Vec3)> {
    let n = (t[1] - t[0]).cross(t[2] - t[0]).try_normalize()?;
    let s0 = n.dot(start - t[0]);
    let s1 = n.dot(end - t[0]);
    // Written so that a NaN end or start is no hit.
    if !(s0 >= 0.0 && s1 < r) {
        return None;
    }
    if s1 > 0.0 {
        return sphere_hit(t, end, r);
    }
    let travel = end - start;
    let length = travel.length();
    if length <= 0.0 {
        return sphere_hit(t, end, r);
    }
    let u = travel / length;
    // Where the centre is one radius in front of the plane.
    let reference = if s0 <= r {
        start
    } else {
        start + travel * ((s0 - r) / (s0 - s1))
    };
    let q = closest_point(t, reference);
    let (a, b) = (q - start, q - end);
    if a.dot(b) <= 0.0 {
        let centre = if a.length_squared() < r * r {
            match a.try_normalize() {
                Some(dir) if a.length_squared() >= ON_TRIANGLE2 => q - dir * r,
                _ => start + n * r,
            }
        } else {
            let along = a.dot(u);
            if along < 0.0 {
                return None;
            }
            let h2 = r * r - (a - u * along).length_squared();
            if h2 <= 0.0 {
                return None;
            }
            let toi = along - h2.sqrt();
            if toi > length {
                return None;
            }
            start + u * toi
        };
        Some((q, centre - end))
    } else {
        // The end passed beside the triangle within reach of `q`: off `q`,
        // never further behind the plane. (The original scales the start's
        // offset here; see the design.)
        let b2 = b.length_squared();
        if b2 >= r * r {
            return None;
        }
        let mut delta = if b2 < ON_TRIANGLE2 {
            n * r
        } else {
            let bl = b2.sqrt();
            b * ((bl - r) / bl)
        };
        let into = delta.dot(n);
        if into < 0.0 {
            delta -= n * into;
        }
        Some((q, delta))
    }
}

/// Of five contacts' weights on a part's particles (four kept, the newcomer
/// last), the one a four-point manifold drops, as the original's level and
/// pair manifolds do: of the two closest in weight space, the one whose own
/// nearest other is closer (the second on a tie). The newcomer (4) may be
/// the one.
pub(crate) fn evicted(all: &[[f32; 4]; 5]) -> usize {
    let gap = |a: usize, b: usize| -> f32 { (0..4).map(|k| (all[a][k] - all[b][k]).powi(2)).sum() };
    let mut pair = (0, 1, f32::MAX);
    for a in 0..all.len() {
        for b in a + 1..all.len() {
            let d = gap(a, b);
            if d < pair.2 {
                pair = (a, b, d);
            }
        }
    }
    let nearest = |i: usize, partner: usize| {
        (0..all.len())
            .filter(|&j| j != i && j != partner)
            .map(|j| gap(i, j))
            .fold(f32::MAX, f32::min)
    };
    let (a, b, _) = pair;
    if nearest(a, b) < nearest(b, a) { a } else { b }
}

/// Friction against the level for a contact point that slipped `slip` this
/// tick: below the limit the slip is cancelled, above it damped by a
/// hundredth of the limit. The limit is in cm² per tick², as the original
/// states it.
pub(crate) fn rub(slip: Vec3, n: Vec3, friction: f32) -> Vec3 {
    let tangent = slip - n * slip.dot(n);
    let limit = 0.5 * friction * (0.01 + 0.99 * n.y * n.y);
    if tangent.length_squared() * 1e4 < limit {
        -tangent
    } else {
        -tangent * (0.01 * limit)
    }
}
