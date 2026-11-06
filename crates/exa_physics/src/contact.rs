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
    /// skips the piece. The plain 1 by default; the running game's level
    /// answers each collider's stored mask ([`crate::StaticColliders`]).
    fn mask(&self, _piece: u32) -> u32 {
        crate::sector::MASK_SMALL
    }

    /// Whether `piece` is the stand-in of a rigid body's meshes, moved with
    /// it: the rays and sweeps meet it, bodies skip it (they meet the
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
    /// bit (0x800000 in its second flag word: 875 mesh nodes ship with it,
    /// and a tile grid's merged cell floors gain it), and skips a disabled
    /// body. None by default; the running game's level answers from the
    /// colliders built from a [`crate::FloorMesh`]
    /// ([`crate::StaticColliders::is_floor`]).
    fn floor_piece(&self, _piece: u32) -> bool {
        false
    }

    /// The triangles of `piece` meeting the box `lo..hi`, each with its
    /// index in the piece's own list (the order the original's mesh ray
    /// walks them in). By default the order [`Level::triangles`] gives.
    fn indexed_triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<(u32, [Vec3; 3])>) {
        let mut plain = Vec::new();
        self.triangles(piece, lo, hi, &mut plain);
        out.extend((0u32..).zip(plain));
    }

    /// Whether the Ground Query drops a hit on `piece`'s triangle 0: the
    /// piece's triangle 0 is its floor node mesh's first triangle, which
    /// the original's query ignores (it takes a mesh's hit only when the
    /// mesh ray returns a triangle index above 0). None by default; the
    /// running game's level answers from the colliders built from a
    /// [`crate::FloorMesh`] that also hold their node mesh's first triangle
    /// ([`crate::StaticColliders::skips_first`]).
    fn floor_skips_first(&self, _piece: u32) -> bool {
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

/// The Ground Query over the level's floor pieces, as the original walks
/// them: each piece's mesh ray ([`mesh_ray`]) runs from `at` straight down
/// within the nearest hit so far (at first `depth`), and its hit counts only
/// on a triangle other than 0 where the piece holds its node mesh's first
/// triangle ([`Level::floor_skips_first`]). A dropped hit on triangle 0
/// also hides that piece's farther faces, as the ray stops there. Returns
/// the height of the nearest kept hit.
fn floor_query<L: Level + ?Sized>(level: &L, at: Vec3, depth: f32) -> Option<f32> {
    let (lo, hi) = (at - Vec3::Y * depth, at);
    let (mut pieces, mut triangles) = (Vec::new(), Vec::new());
    level.pieces(lo, hi, &mut pieces);
    let mut nearest = depth;
    let mut found = false;
    for &piece in &pieces {
        if !level.floor_piece(piece) {
            continue;
        }
        triangles.clear();
        level.indexed_triangles(piece, lo, hi, &mut triangles);
        triangles.sort_unstable_by_key(|(i, _)| *i);
        let Some((index, t)) = mesh_ray(&triangles, at, Vec3::NEG_Y, nearest) else {
            continue;
        };
        if index == 0 && level.floor_skips_first(piece) {
            continue;
        }
        nearest = t;
        found = true;
    }
    found.then_some(at.y - nearest)
}

/// The engine's mesh ray determinant floor (cm²): a face whose
/// determinant is under it is edge-on or turned away.
const MESH_RAY_EPSILON_CM2: f32 = f32::from_bits(0x33ff_ffd7);
/// [`MESH_RAY_EPSILON_CM2`] in m² (the determinant scales with the
/// square of the length unit).
const MESH_RAY_EPSILON: f32 = MESH_RAY_EPSILON_CM2 * 1e-4;

/// The engine's dot product in a mesh ray: x, then y, then z.
fn ray_dot(a: Vec3, b: Vec3) -> f32 {
    (a.x * b.x + a.y * b.y) + a.z * b.z
}

/// The engine's plain mesh ray (a mesh without a k-d tree): every triangle
/// in index order, front faces only (the determinant at least the engine's
/// epsilon), the barycentric bounds unnormalised against the determinant,
/// and a hit strictly inside `(0, limit)` and strictly nearer than the
/// last one taken. Returns the index of the nearest triangle met and its
/// distance along `dir` (unit), or `None`.
pub fn mesh_ray(
    triangles: &[(u32, [Vec3; 3])],
    origin: Vec3,
    dir: Vec3,
    limit: f32,
) -> Option<(u32, f32)> {
    let mut limit = limit;
    let mut hit = None;
    for &(index, [v0, v1, v2]) in triangles {
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let p = dir.cross(e2);
        let det = ray_dot(p, e1);
        // An unordered determinant (NaN) fails the compare too.
        if det.is_nan() || det < MESH_RAY_EPSILON {
            continue;
        }
        let s = origin - v0;
        let u = ray_dot(s, p);
        if !(0.0 <= u && u <= det) {
            continue;
        }
        let q = s.cross(e1);
        let v = ray_dot(dir, q);
        if !(0.0 <= v && u + v <= det) {
            continue;
        }
        let t = ray_dot(e2, q) / det;
        if 0.0 < t && t < limit {
            limit = t;
            hit = Some((index, t));
        }
    }
    hit
}

/// The original's line cast (the start of play's, a level load's and the
/// arena's floor probe): a ray from `from` along `dir` (unit) at most
/// `reach`, against every level piece and rigid body stand-in, each with
/// the plain mesh ray within the nearest hit so far; any triangle counts.
/// Returns the point it meets.
pub fn line_cast<L: Level + ?Sized>(level: &L, from: Vec3, dir: Vec3, reach: f32) -> Option<Vec3> {
    if !(reach > 0.0 && from.is_finite() && dir.is_finite()) {
        return None;
    }
    let to = from + dir * reach;
    let (lo, hi) = (from.min(to), from.max(to));
    let (mut pieces, mut triangles) = (Vec::new(), Vec::new());
    level.pieces(lo, hi, &mut pieces);
    let mut nearest = reach;
    let mut found = false;
    for &piece in &pieces {
        triangles.clear();
        level.indexed_triangles(piece, lo, hi, &mut triangles);
        triangles.sort_unstable_by_key(|(i, _)| *i);
        if let Some((_, t)) = mesh_ray(&triangles, from, dir, nearest) {
            nearest = t;
            found = true;
        }
    }
    found.then(|| from + dir * nearest)
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

    /// The triangles with their index in the collider's mesh (the order
    /// the mesh was built in).
    fn indexed_triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<(u32, [Vec3; 3])>) {
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
            out.push((i, [t.a, t.b, t.c].map(|v| pose.transform_point(v))));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A square floor at `y` over `[-1, 1]²`, two upward faces.
    fn square(y: f32) -> [[Vec3; 3]; 2] {
        let p = |x: f32, z: f32| Vec3::new(x, y, z);
        [
            [p(-1.0, -1.0), p(-1.0, 1.0), p(1.0, 1.0)],
            [p(-1.0, -1.0), p(1.0, 1.0), p(1.0, -1.0)],
        ]
    }

    /// Floor pieces, each one mesh; `first` marks the pieces that hold
    /// their node mesh's triangle 0.
    struct Floors {
        pieces: Vec<Vec<[Vec3; 3]>>,
        first: Vec<bool>,
    }

    impl Level for Floors {
        fn pieces(&self, _: Vec3, _: Vec3, out: &mut Vec<u32>) {
            out.extend(0..self.pieces.len() as u32);
        }
        fn triangles(&self, piece: u32, _: Vec3, _: Vec3, out: &mut Vec<[Vec3; 3]>) {
            out.extend_from_slice(&self.pieces[piece as usize]);
        }
        fn floor_piece(&self, _: u32) -> bool {
            true
        }
        fn floor_skips_first(&self, piece: u32) -> bool {
            self.first[piece as usize]
        }
    }

    /// The plain mesh ray: front faces only, the nearest strictly, the first
    /// in index order on a tie, and a hit strictly inside the limit.
    #[test]
    fn the_mesh_ray_takes_the_nearest_front_face() {
        let [a, b] = square(0.0);
        let [c, _] = square(0.5);
        let tris = vec![(0, a), (1, b), (2, c)];
        let down = Vec3::NEG_Y;
        // Over the first face: the upper square's face 2 is nearer.
        let (i, t) = mesh_ray(&tris, Vec3::new(-0.5, 1.0, 0.5), down, 2.0).unwrap();
        assert_eq!(i, 2);
        assert!((t - 0.5).abs() < 1e-6);
        // Limited to 0.5: the hit at exactly 0.5 is not inside.
        assert_eq!(
            mesh_ray(&tris, Vec3::new(-0.5, 1.0, 0.5), down, 0.5).map(|h| h.0),
            None
        );
        // From below, the faces are turned away.
        assert_eq!(
            mesh_ray(&tris, Vec3::new(-0.5, -1.0, 0.5), Vec3::Y, 5.0),
            None
        );
        // Two faces at one distance: the first in index order.
        let tied = vec![(4, a), (7, a)];
        assert_eq!(
            mesh_ray(&tied, Vec3::new(-0.5, 1.0, 0.5), down, 2.0).map(|h| h.0),
            Some(4)
        );
    }

    /// The Ground Query drops a floor mesh's hit on its triangle 0, and that
    /// hit hides the mesh's farther faces; another mesh's hit stands, and a
    /// mesh that does not hold its node's triangle 0 keeps it.
    #[test]
    fn the_ground_query_drops_a_hit_on_triangle_zero() {
        let over_first = Vec3::new(-0.5, 0.3, 0.5);
        let over_second = Vec3::new(0.5, 0.3, -0.5);
        // One mesh: a square at 0 under a square at 0.2 whose triangle 0
        // covers the point.
        let [low0, low1] = square(0.0);
        let [high0, high1] = square(0.2);
        let one = Floors {
            pieces: vec![vec![high0, high1, low0, low1]],
            first: vec![true],
        };
        assert_eq!(one.floor_below(over_first, 0.6), None);
        let h = one.floor_below(over_second, 0.6).unwrap();
        assert!((h - 0.2).abs() < 1e-6, "{h}");
        // The low square as a mesh of its own still answers.
        let two = Floors {
            pieces: vec![vec![high0, high1], vec![low1, low0]],
            first: vec![true, true],
        };
        let h = two.floor_below(over_first, 0.6).unwrap();
        assert!(h.abs() < 1e-6, "{h}");
        // A part that does not hold its node's triangle 0 keeps the hit.
        let kept = Floors {
            pieces: vec![vec![high0, high1, low0, low1]],
            first: vec![false],
        };
        let h = kept.floor_below(over_first, 0.6).unwrap();
        assert!((h - 0.2).abs() < 1e-6, "{h}");
    }

    /// The line cast keeps any triangle and returns the point it meets.
    #[test]
    fn the_line_cast_meets_the_nearest_front_face() {
        let [a, b] = square(0.0);
        let [c, d] = square(2.0);
        let level = Floors {
            pieces: vec![vec![a, b], vec![c, d]],
            first: vec![true, true],
        };
        let at = line_cast(&level, Vec3::new(-0.5, 10.0, 0.5), Vec3::NEG_Y, 30.0).unwrap();
        assert!((at - Vec3::new(-0.5, 2.0, 0.5)).length() < 1e-5, "{at}");
        assert_eq!(
            line_cast(&level, Vec3::new(5.0, 10.0, 0.5), Vec3::NEG_Y, 30.0),
            None
        );
        assert_eq!(
            line_cast(&level, Vec3::new(-0.5, 10.0, 0.5), Vec3::NEG_Y, 7.0),
            None
        );
    }
}
