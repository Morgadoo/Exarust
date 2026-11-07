//! A body node's CPU triangles behind the original's triangle kd-tree: the
//! mesh queries a part answers in the pair stage, the Height Query and the
//! picks.
//!
//! # What the original does
//!
//! The scene loader builds a mesh node's triangle records (plane, vertex
//! pointers and the closest-point terms) when the node carries the
//! collision flag `0x200000` and not the opt-out `0x4000`, and then hands
//! the mesh to the kd-tree builder thread when it has more than 10 and
//! fewer than 100 000 triangles ([`TREE_MIN_TRIANGLES`],
//! [`TREE_MAX_TRIANGLES`]). The thread builds the tree and installs its
//! query callbacks into the mesh and all its instances; until then (and
//! for a mesh outside the bounds, for ever) the mesh's own brute-force
//! callbacks answer, which test every triangle in order. A character's
//! limbs are such nodes of its body model (the naked body part; the
//! garments do not change the shape), and so is a weapon's merged node.
//! The shape builders (a limb's, a body root's and a weapon's fit) build
//! the triangle records and a tree for the mesh they fit when it has none,
//! whatever its size, so every character limb carries one.
//! The tree is built once, in the node's frame; a skinned limb is queried
//! in its part's frame, so nothing is rebuilt as the body moves.
//!
//! The tree's sphere query visits the leaves the sphere's box reaches
//! ([`KdTree::sphere_leaves`]); the swept query walks the leaves along the
//! swept sphere with its record kept live ([`KdTree::sweep_live`]); the
//! Height Query's column asks the nearest swept walk
//! ([`KdTree::sweep_nearest`]); a pick asks the ray walk ([`KdTree::ray`]).
//! The per-triangle tests are the mesh's own, so only the order the
//! triangles are tested in differs from the brute-force path.
//!
//! # Exarust
//!
//! Exarust's triangles are in metres; the tree is built over the same
//! triangles in the original's centimetres, so its split limits (the
//! 0.1 leaf extent, the 0.01 box padding) mean what they mean in the
//! original, and every query is scaled into it. Trees are built when a
//! body takes its triangles, on the calling thread, and shared between the
//! bodies that share a triangle list ([`shared_tree`]).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use bevy::math::{Affine3A, Vec3};

use crate::kd_tree::{KdTree, Sweep, SweptRecord, Triangle};

/// A mesh gets a tree only with more triangles than this.
pub const TREE_MIN_TRIANGLES: usize = 10;
/// A mesh gets a tree only with fewer triangles than this.
pub const TREE_MAX_TRIANGLES: usize = 100_000;
/// The tree's units per metre: the original's centimetres.
pub const TREE_SCALE: f32 = 100.0;

/// Whether the original queues a mesh of `count` triangles for a tree.
pub fn takes_tree(count: usize) -> bool {
    count > TREE_MIN_TRIANGLES && count < TREE_MAX_TRIANGLES
}

/// A mesh's triangle kd-tree, in centimetres over the mesh's own frame.
#[derive(Debug, Clone)]
pub struct MeshTree {
    pub tree: KdTree,
}

impl MeshTree {
    /// The tree over `triangles` (metres), when the original would build
    /// one ([`takes_tree`]).
    pub fn build(triangles: &[[Vec3; 3]]) -> Option<Self> {
        if !takes_tree(triangles.len()) {
            return None;
        }
        Self::build_any(triangles)
    }

    /// The tree over any non-empty list, as the shape builders make one
    /// for the mesh they fit (a limb's, a body root's or a weapon's) when
    /// it has none, whatever its triangle count.
    pub fn build_any(triangles: &[[Vec3; 3]]) -> Option<Self> {
        if triangles.is_empty() {
            return None;
        }
        let mesh: Vec<Triangle> = triangles
            .iter()
            .map(|t| Triangle::new(t[0] * TREE_SCALE, t[1] * TREE_SCALE, t[2] * TREE_SCALE))
            .collect();
        Some(Self {
            tree: KdTree::build(&mesh, false),
        })
    }

    /// The sphere query: `leaf` gets the triangle indices of each leaf the
    /// sphere (metres) reaches and may move it ([`KdTree::sphere_leaves`]).
    pub fn sphere(
        &self,
        sphere: &mut (Vec3, f32),
        leaf: impl FnMut(&mut (Vec3, f32), &[u32]) -> bool,
    ) -> bool {
        self.tree.sphere_leaves(sphere, TREE_SCALE, leaf)
    }

    /// The swept query on a live record in metres ([`KdTree::sweep_live`]).
    pub fn sweep(
        &self,
        record: &mut SweptRecord,
        leaf: impl FnMut(&mut SweptRecord, &[u32]) -> bool,
    ) -> bool {
        self.tree.sweep_live(record, TREE_SCALE, leaf)
    }

    /// The first front face along a ray (metres): the triangle and the
    /// distance, within `max`.
    pub fn ray(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<(u32, f32)> {
        self.tree
            .ray(origin * TREE_SCALE, direction, max * TREE_SCALE)
            .map(|hit| (hit.triangle, hit.distance / TREE_SCALE))
    }

    /// The nearest drop of a sphere of `radius` centred at `centre`
    /// straight down by at most `depth` (metres) onto the mesh: the column
    /// walk of the original's Height Query in its mesh mode, each leaf
    /// answering with its nearest face ([`crate::contact::drop_onto`]).
    pub fn drop_onto(
        &self,
        triangles: &[[Vec3; 3]],
        centre: Vec3,
        radius: f32,
        depth: f32,
    ) -> Option<f32> {
        let sweep = Sweep {
            origin: centre * TREE_SCALE,
            direction: Vec3::NEG_Y,
            radius: radius * TREE_SCALE,
        };
        self.tree
            .sweep_nearest(&sweep, depth * TREE_SCALE, |_, ids, limit| {
                let mut found = None;
                for &id in ids {
                    let Some(t) = triangles.get(id as usize) else {
                        continue;
                    };
                    if let Some(d) =
                        crate::contact::drop_onto(t, centre, radius, *limit / TREE_SCALE)
                        && d * TREE_SCALE < *limit
                    {
                        *limit = d * TREE_SCALE;
                        found = Some(id);
                    }
                }
                found
            })
            .map(|hit| hit.distance / TREE_SCALE)
    }
}

/// The first front face of a node mesh along a unit ray (metres, in the
/// mesh's own frame), within `max`: the original's mesh ray query, through
/// the mesh's kd-tree when it has one, else every triangle in the file's
/// order, the nearest kept. Each triangle is tested in centimetres, as the
/// tree stores them. Returns the triangle and the distance (m).
pub fn ray_mesh(
    triangles: &[[Vec3; 3]],
    tree: Option<&MeshTree>,
    origin: Vec3,
    direction: Vec3,
    max: f32,
) -> Option<(u32, f32)> {
    if let Some(tree) = tree {
        return tree.ray(origin, direction, max);
    }
    let from = origin * TREE_SCALE;
    let mut best: Option<(u32, f32)> = None;
    let mut reach = max * TREE_SCALE;
    for (i, t) in triangles.iter().enumerate() {
        let tri = Triangle::new(t[0] * TREE_SCALE, t[1] * TREE_SCALE, t[2] * TREE_SCALE);
        if let Some(d) = crate::kd_tree::ray_triangle(&tri, from, direction, reach) {
            reach = d;
            best = Some((i as u32, d / TREE_SCALE));
        }
    }
    best
}

/// A node mesh's sphere query (metres, in the mesh's own frame), as an
/// effect's contact asks it: every front face the sphere reaches gives its
/// closest point to `on_point`, which may move the sphere (the next face
/// sees the moved sphere). Through the mesh's kd-tree when it has one (the
/// leaves the sphere's box reaches, in the tree's order), else every
/// triangle in the file's order. The face test is the pair stage's
/// (the sphere's centre in front of the face, within its radius). Returns
/// whether a face was met.
pub fn sphere_contacts(
    triangles: &[[Vec3; 3]],
    tree: Option<&MeshTree>,
    sphere: &mut (Vec3, f32),
    mut on_point: impl FnMut(&mut (Vec3, f32), Vec3),
) -> bool {
    let mut face = |s: &mut (Vec3, f32), t: &[Vec3; 3]| -> bool {
        match crate::contact::sphere_hit(t, s.0, s.1) {
            Some((q, _)) => {
                on_point(s, q);
                true
            }
            None => false,
        }
    };
    if let Some(tree) = tree {
        return tree.sphere(sphere, |s, ids| {
            let mut hit = false;
            for &id in ids {
                if let Some(t) = triangles.get(id as usize) {
                    hit |= face(s, t);
                }
            }
            hit
        });
    }
    let mut hit = false;
    for t in triangles {
        hit |= face(sphere, t);
    }
    hit
}

/// The signed volume a closed mesh encloses (in its units cubed): positive
/// when its fronts ((b − a) × (c − a)) face out, as the mesh queries need
/// (a debug check of a mesh's winding).
pub fn signed_volume(triangles: &[[Vec3; 3]]) -> f32 {
    triangles
        .iter()
        .map(|t| t[0].dot(t[1].cross(t[2])) / 6.0)
        .sum()
}

/// The trees built so far, keyed by the triangle list they were built from
/// (its address, checked against a weak handle so a reused address is not
/// mistaken for the old list).
type TreeCache = HashMap<usize, (Weak<Vec<[Vec3; 3]>>, Option<Arc<MeshTree>>)>;

fn cache() -> &'static Mutex<TreeCache> {
    static CACHE: OnceLock<Mutex<TreeCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The tree of a shared triangle list, built the first time it is asked
/// for and shared by every body that holds the same list (the original
/// installs one tree into a mesh and all its instances). `None` for a list
/// the original leaves to the brute-force callbacks.
pub fn shared_tree(triangles: &Arc<Vec<[Vec3; 3]>>) -> Option<Arc<MeshTree>> {
    if !takes_tree(triangles.len()) {
        return None;
    }
    cached_tree(triangles)
}

/// The tree a shape builder makes for the mesh it fits: built for any
/// non-empty list ([`MeshTree::build_any`]), shared as [`shared_tree`]'s.
pub fn shape_tree(triangles: &Arc<Vec<[Vec3; 3]>>) -> Option<Arc<MeshTree>> {
    if triangles.is_empty() {
        return None;
    }
    cached_tree(triangles)
}

fn cached_tree(triangles: &Arc<Vec<[Vec3; 3]>>) -> Option<Arc<MeshTree>> {
    let key = Arc::as_ptr(triangles) as usize;
    if let Ok(cache) = cache().lock()
        && let Some((weak, tree)) = cache.get(&key)
        && weak.upgrade().is_some_and(|t| Arc::ptr_eq(&t, triangles))
    {
        return tree.clone();
    }
    let tree = MeshTree::build_any(triangles).map(Arc::new);
    if let Ok(mut cache) = cache().lock() {
        cache.retain(|_, (weak, _)| weak.strong_count() > 0);
        cache.insert(key, (Arc::downgrade(triangles), tree.clone()));
    }
    tree
}

/// The Height Query's answer from one body mesh in the original's mesh
/// mode: a sphere of `radius` whose lowest point starts at `at` (world,
/// metres), swept straight down by at most `depth`, against `triangles`
/// placed by `world` (a rigid frame). The segment must first meet the
/// mesh's box (`centre`, `half`, in its frame, grown by the radius) and its
/// bounding sphere. Returns the drop distance.
#[allow(clippy::too_many_arguments)]
pub fn body_drop(
    triangles: &[[Vec3; 3]],
    tree: Option<&MeshTree>,
    world: &Affine3A,
    (centre, half, bound): (Vec3, Vec3, f32),
    at: Vec3,
    radius: f32,
    depth: f32,
) -> Option<f32> {
    let to_local = world.inverse();
    let start = to_local.transform_point3(at + Vec3::Y * radius);
    let down = to_local.transform_vector3(Vec3::NEG_Y);
    let end = start + down * depth;
    // The segment against the box grown by the radius, then the swept
    // sphere against the bounding sphere.
    let sweep = crate::intersect::Sweep::new(start, end);
    if !crate::intersect::swept_sphere_aabb_hit(&sweep, radius, centre, half) {
        return None;
    }
    if crate::intersect::sphere_segment_separated(&sweep, radius, centre, bound) {
        return None;
    }
    if down.distance_squared(Vec3::NEG_Y) > 1e-6 {
        // A tilted body: the column is not straight down its frame, so the
        // drop is taken over the triangles turned into the world.
        let placed: Vec<[Vec3; 3]> = triangles
            .iter()
            .map(|t| t.map(|v| world.transform_point3(v)))
            .collect();
        return lowest_drop(&placed, at + Vec3::Y * radius, radius, depth);
    }
    match tree {
        Some(tree) => tree.drop_onto(triangles, start, radius, depth),
        None => lowest_drop(triangles, start, radius, depth),
    }
}

/// The brute-force column: every triangle in order, the nearest drop.
fn lowest_drop(triangles: &[[Vec3; 3]], centre: Vec3, radius: f32, depth: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for t in triangles {
        if let Some(d) = crate::contact::drop_onto(t, centre, radius, best.unwrap_or(depth))
            && best.is_none_or(|b| d < b)
        {
            best = Some(d);
        }
    }
    best
}

#[cfg(test)]
#[path = "mesh_tree_test.rs"]
mod tests;
