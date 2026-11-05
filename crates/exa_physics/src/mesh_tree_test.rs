//! Tests of the mesh trees on synthetic triangles: the tree answers what
//! the brute-force callbacks answer, in the original's leaf order.

use super::*;
use crate::contact::{sphere_hit, swept_hit};
use crate::kd_tree::Node;

/// A cube of side `2 · half` about `centre`, each face cut into `n × n`
/// squares of two triangles, fronts facing out ((b − a) × (c − a)).
fn cube(centre: Vec3, half: f32, n: usize) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
    for (k, &normal) in axes.iter().enumerate() {
        let u = axes[(k + 1) % 3];
        let v = axes[(k + 2) % 3];
        for sign in [1.0f32, -1.0] {
            let n_out = normal * sign;
            for i in 0..n {
                for j in 0..n {
                    let at = |a: usize, b: usize| {
                        let s = a as f32 / n as f32 * 2.0 - 1.0;
                        let t = b as f32 / n as f32 * 2.0 - 1.0;
                        centre + (n_out + u * s + v * t) * half
                    };
                    let (p00, p10, p01, p11) =
                        (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1));
                    for tri in [[p00, p10, p11], [p00, p11, p01]] {
                        let front = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
                        if front.dot(n_out) > 0.0 {
                            out.push(tri);
                        } else {
                            out.push([tri[0], tri[2], tri[1]]);
                        }
                    }
                }
            }
        }
    }
    out
}

/// A small, cheap pseudo-random sequence.
fn rng(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 8) as f32 / (1u32 << 24) as f32
}

fn point(seed: &mut u32, spread: f32) -> Vec3 {
    Vec3::new(rng(seed) - 0.5, rng(seed) - 0.5, rng(seed) - 0.5) * spread
}

#[test]
fn a_tree_is_built_only_between_ten_and_a_hundred_thousand_triangles() {
    assert!(!takes_tree(10));
    assert!(takes_tree(11));
    assert!(takes_tree(99_999));
    assert!(!takes_tree(100_000));
    let tri = [Vec3::ZERO, Vec3::X, Vec3::Z];
    assert!(MeshTree::build(&[tri; 10]).is_none());
    assert!(MeshTree::build(&[tri; 11]).is_some());
}

#[test]
fn a_shared_list_shares_its_tree() {
    let list = Arc::new(cube(Vec3::ZERO, 0.1, 4));
    let a = shared_tree(&list).unwrap();
    let b = shared_tree(&list).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    // Another list with the same triangles gets its own tree.
    let other = Arc::new(cube(Vec3::ZERO, 0.1, 4));
    assert!(!Arc::ptr_eq(&a, &shared_tree(&other).unwrap()));
    assert!(shared_tree(&Arc::new(vec![[Vec3::ZERO, Vec3::X, Vec3::Z]; 3])).is_none());
}

/// The sphere query finds exactly the triangles the brute-force callback
/// finds (each once) for spheres all about a limb-sized mesh.
#[test]
fn the_sphere_query_hits_what_the_brute_force_hits() {
    let mesh = cube(Vec3::new(0.02, 0.15, -0.01), 0.06, 8);
    let tree = MeshTree::build(&mesh).unwrap();
    let mut seed = 7;
    let mut tested = 0;
    for _ in 0..400 {
        let c = Vec3::new(0.02, 0.15, -0.01) + point(&mut seed, 0.2);
        let r = 0.01 + rng(&mut seed) * 0.04;
        let mut brute: Vec<u32> = (0..mesh.len() as u32)
            .filter(|&i| sphere_hit(&mesh[i as usize], c, r).is_some())
            .collect();
        let mut found = Vec::new();
        let mut sphere = (c, r);
        tree.sphere(&mut sphere, |s, ids| {
            let mut hit = false;
            for &id in ids {
                if sphere_hit(&mesh[id as usize], s.0, s.1).is_some() {
                    found.push(id);
                    hit = true;
                }
            }
            hit
        });
        found.sort_unstable();
        found.dedup();
        brute.sort_unstable();
        tested += brute.len();
        assert_eq!(found, brute, "sphere {c} r {r}");
    }
    assert!(tested > 100);
}

/// The leaf order: the first child before the second, the deferred second
/// children last in, first out; a sphere past a plane skips its first
/// child, one short of it skips its second.
#[test]
fn the_sphere_query_walks_first_children_first() {
    // Root splits x at 0: first = node 1 (a split on x at -1 with leaves
    // 3 and 4), second = node 2 (leaf).
    let mut tree = KdTree {
        nodes: vec![
            Node::split(0, 1, 0.0),
            Node::split(0, 2, -1.0),
            Node::leaf(1, 0),
            Node::leaf(1, 1),
            Node::leaf(1, 2),
        ],
        triangles: vec![2, 0, 1],
        ..KdTree::default()
    };
    tree.mesh = vec![Triangle::new(Vec3::ZERO, Vec3::X, Vec3::Z); 3];
    let order = |c: f32, r: f32| {
        let mut seen = Vec::new();
        tree.sphere_leaves(&mut (Vec3::new(c, 0.0, 0.0), r), 1.0, |_, ids| {
            seen.extend_from_slice(ids);
            false
        });
        seen
    };
    // Reaching every plane: node 3, node 4, then node 2.
    assert_eq!(order(-0.5, 2.0), vec![0, 1, 2]);
    // Past the root's plane: the second child only.
    assert_eq!(order(5.0, 1.0), vec![2]);
    // Short of the root's plane and past the inner one's: node 4 only.
    assert_eq!(order(-0.5, 0.25), vec![1]);
    // A NaN centre goes to the first children only.
    assert_eq!(order(f32::NAN, 1.0), vec![0]);
}

/// The swept query finds what the brute-force swept callback finds when
/// no hit moves the sphere.
#[test]
fn the_swept_query_hits_what_the_brute_force_hits() {
    let mesh = cube(Vec3::ZERO, 0.08, 6);
    let tree = MeshTree::build(&mesh).unwrap();
    let mut seed = 99;
    let mut tested = 0;
    for _ in 0..400 {
        let start = point(&mut seed, 0.4);
        let end = point(&mut seed, 0.4);
        let r = 0.005 + rng(&mut seed) * 0.03;
        let mut brute: Vec<u32> = (0..mesh.len() as u32)
            .filter(|&i| swept_hit(&mesh[i as usize], start, end, r).is_some())
            .collect();
        let mut record = SweptRecord::new(start, end, r);
        let mut found = Vec::new();
        tree.sweep(&mut record, |rec, ids| {
            let mut hit = false;
            for &id in ids {
                if swept_hit(&mesh[id as usize], rec.start, rec.end, rec.radius).is_some() {
                    found.push(id);
                    hit = true;
                }
            }
            hit
        });
        found.sort_unstable();
        found.dedup();
        brute.sort_unstable();
        tested += brute.len();
        assert_eq!(found, brute, "{start} -> {end} r {r}");
    }
    assert!(tested > 50);
}

/// A leaf that moves the end refreshes the record: the walk goes on with
/// the new length and direction, and the record keeps the best length.
#[test]
fn a_refreshed_sweep_keeps_the_shorter_length() {
    let mesh = cube(Vec3::ZERO, 0.1, 6);
    let tree = MeshTree::build(&mesh).unwrap();
    let start = Vec3::new(0.0, 0.5, 0.0);
    let mut record = SweptRecord::new(start, Vec3::new(0.0, -0.5, 0.0), 0.02);
    assert!((record.length - 1.0).abs() < 1e-6);
    let mut first = true;
    let hit = tree.sweep(&mut record, |rec, ids| {
        let mut hit = false;
        for &id in ids {
            if let Some((_, delta)) = swept_hit(&mesh[id as usize], rec.start, rec.end, rec.radius)
                && first
            {
                first = false;
                rec.end += delta;
                rec.refresh();
                hit = true;
            }
        }
        hit
    });
    assert!(hit);
    // The top face stops the sphere one radius over it.
    assert!((record.end.y - 0.12).abs() < 1e-4, "{}", record.end);
    assert!(
        (record.length - (0.5 - 0.12)).abs() < 1e-4,
        "{}",
        record.length
    );
}

/// The ray walk gives the brute-force nearest front face.
#[test]
fn the_ray_finds_the_nearest_front_face() {
    let mesh = cube(Vec3::new(1.0, 0.5, 0.0), 0.1, 5);
    let tree = MeshTree::build(&mesh).unwrap();
    let mut seed = 3;
    let mut hits = 0;
    for _ in 0..300 {
        let origin = Vec3::new(1.0, 0.5, 0.0) + point(&mut seed, 1.0);
        let towards = Vec3::new(1.0, 0.5, 0.0) + point(&mut seed, 0.15);
        let dir = (towards - origin).normalize();
        let brute = mesh
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let n = (t[1] - t[0]).cross(t[2] - t[0]);
                if n.dot(dir) >= 0.0 {
                    return None;
                }
                let s = (t[0] - origin).dot(n) / dir.dot(n);
                let p = origin + dir * s;
                let inside = (0..3).all(|k| (t[(k + 1) % 3] - t[k]).cross(p - t[k]).dot(n) >= 0.0);
                (s > 0.0 && inside).then_some((i as u32, s))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let found = tree.ray(origin, dir, 10.0);
        match (brute, found) {
            (Some((_, d)), Some((_, t))) => {
                hits += 1;
                assert!((d - t).abs() < 1e-4, "{d} vs {t}");
            }
            (None, None) => {}
            (b, f) => {
                // Grazing an edge may go either way.
                let d = b.map_or_else(|| f.unwrap().1, |b| b.1);
                let p = origin + dir * d;
                let local = (p - Vec3::new(1.0, 0.5, 0.0)).abs();
                let on_edge = [local.x, local.y, local.z]
                    .iter()
                    .filter(|&&c| (c - 0.1).abs() < 1e-3)
                    .count()
                    >= 2;
                assert!(on_edge, "{b:?} vs {f:?}");
            }
        }
    }
    assert!(hits > 50);
}

/// The Height Query's column over a body mesh lands where the brute-force
/// column lands; a body turned about the vertical answers the same, and
/// one beside the column is passed over.
#[test]
fn the_column_drop_matches_the_brute_force() {
    let mesh = cube(Vec3::new(0.0, 0.2, 0.0), 0.2, 6);
    let tree = MeshTree::build(&mesh).unwrap();
    let bounds = (Vec3::new(0.0, 0.2, 0.0), Vec3::splat(0.2), 0.35);
    let mut seed = 11;
    for _ in 0..100 {
        let at = Vec3::new(rng(&mut seed) - 0.5, 1.0, rng(&mut seed) - 0.5) * 0.5 + Vec3::Y;
        let brute = lowest_drop(&mesh, at + Vec3::Y * 0.1, 0.1, 5.0);
        let found = tree.drop_onto(&mesh, at + Vec3::Y * 0.1, 0.1, 5.0);
        match (brute, found) {
            (Some(b), Some(f)) => assert!((b - f).abs() < 1e-5),
            (b, f) => assert_eq!(b, f),
        }
        let world = Affine3A::from_rotation_y(0.0);
        let body = body_drop(&mesh, Some(&tree), &world, bounds, at, 0.1, 5.0);
        assert_eq!(body.is_some(), found.is_some());
    }
    // Straight over the top: it stops on the top face at 0.4.
    let at = Vec3::new(0.0, 1.0, 0.0);
    let d = body_drop(
        &mesh,
        Some(&tree),
        &Affine3A::IDENTITY,
        bounds,
        at,
        0.1,
        5.0,
    )
    .unwrap();
    assert!((at.y - d - 0.4).abs() < 1e-5, "{d}");
    // Turned about y and moved: the same height.
    let world = Affine3A::from_rotation_translation(
        bevy::math::Quat::from_rotation_y(0.7),
        Vec3::new(3.0, 0.0, 0.0),
    );
    let d = body_drop(
        &mesh,
        Some(&tree),
        &world,
        bounds,
        at + Vec3::X * 3.0,
        0.1,
        5.0,
    )
    .unwrap();
    assert!((at.y - d - 0.4).abs() < 1e-4, "{d}");
    // Beside it: missed by the box test.
    assert!(body_drop(&mesh, Some(&tree), &world, bounds, at, 0.1, 5.0).is_none());
}
