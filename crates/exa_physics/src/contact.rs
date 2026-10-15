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
}

/// The level colliders of the physics world: every fixed triangle mesh.
impl Level for rapier3d::prelude::PhysicsWorld {
    fn pieces(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) {
        use rapier3d::prelude::*;
        out.extend(
            self.intersect_aabb_conservative(Aabb::new(lo, hi), QueryFilter::only_fixed())
                .filter(|(_, c)| c.shape().as_trimesh().is_some())
                .map(|(h, _)| h.into_raw_parts().0),
        );
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
