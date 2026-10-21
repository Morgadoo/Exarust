//! Tests of the triangle kd-tree and the terrain block tree, on synthetic
//! triangles only.

use super::*;

fn quad() -> Vec<Triangle> {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(2.0, 0.0, 0.0);
    let c = Vec3::new(0.0, 0.0, 2.0);
    let d = Vec3::new(2.0, 0.0, 2.0);
    vec![Triangle::new(a, b, c), Triangle::new(b, d, c)]
}

#[test]
fn nearest_ray_hits_front_face() {
    let tree = KdTree::build(&quad(), false);
    // From below, along +y: the quad's normal points down, so it is a
    // front face.
    let hit = tree
        .ray(Vec3::new(0.25, -5.0, 0.25), Vec3::Y, 10.0)
        .expect("hit");
    assert_eq!(hit.triangle, 0);
    assert_eq!(hit.distance, 5.0);
    // From above the winding is a back face: no hit.
    assert!(
        tree.ray(Vec3::new(0.25, 5.0, 0.25), Vec3::NEG_Y, 10.0)
            .is_none()
    );
    assert!(tree.ray(Vec3::new(5.0, 5.0, 5.0), Vec3::Y, 10.0).is_none());
}

#[test]
fn segment_includes_far_end() {
    let tree = KdTree::build(&quad(), false);
    let hit = tree
        .segment(Vec3::new(0.25, 5.0, 0.25), Vec3::new(0.0, -5.0, 0.0), 1.0)
        .expect("hit");
    assert_eq!(hit.triangle, 0);
    assert_eq!(hit.distance, 1.0);
    assert!(
        tree.segment(Vec3::new(0.25, 5.0, 0.25), Vec3::new(0.0, -4.0, 0.0), 1.0)
            .is_none()
    );
}

#[test]
fn any_ray_uses_first_face() {
    let tree = KdTree::build(&quad(), false);
    assert_eq!(
        tree.ray_any(Vec3::new(0.25, -5.0, 0.25), Vec3::Y, 10.0),
        Some(0)
    );
    assert_eq!(
        tree.ray_any(Vec3::new(0.25, 5.0, 0.25), Vec3::NEG_Y, 10.0),
        None
    );
}

#[test]
fn point_queries_use_the_radius() {
    let tree = KdTree::build(&quad(), false);
    let p = Vec3::new(0.25, 0.5, 0.25);
    assert_eq!(tree.sphere_overlap(p, 0.5, 100.0), Some(0));
    assert_eq!(tree.sphere_overlap(p, 0.49, 100.0), None);
    let hit = tree.closest(p, 10.0, 100.0).expect("closest");
    assert_eq!(hit.triangle, 0);
    assert_eq!(hit.distance, 0.5);
    assert_eq!(
        tree.capsule_overlap(Vec3::new(0.25, -1.0, 0.25), Vec3::new(0.25, 1.0, 0.25), 0.1),
        Some(0)
    );
}

#[test]
fn build_puts_every_triangle_in_one_leaf() {
    let mut mesh = Vec::new();
    for j in 0..20 {
        for i in 0..30 {
            let x = i as f32 * 3.0;
            let z = j as f32 * 3.0;
            mesh.push(Triangle::new(
                Vec3::new(x, 0.0, z),
                Vec3::new(x + 1.0, 0.0, z),
                Vec3::new(x, 0.0, z + 1.0),
            ));
        }
    }
    let tree = KdTree::build(&mesh, false);
    assert!(!tree.nodes[0].is_leaf(), "600 triangles split");
    let mut seen = vec![0u32; mesh.len()];
    for node in &tree.nodes {
        if node.is_leaf() {
            let start = node.triangle_start();
            for &tri in &tree.triangles[start..start + node.triangle_count() as usize] {
                seen[tri as usize] += 1;
            }
        }
    }
    assert_eq!(seen, vec![1; mesh.len()]);
    // The split plane is inside the root bounds and on the split axis.
    let root = tree.nodes[0];
    assert!(root.axis() < 3);
    assert!(root.value >= tree.min[root.axis()] && root.value <= tree.max[root.axis()]);
}

#[test]
fn flat_trees_never_split_y() {
    let mut mesh = Vec::new();
    for i in 0..16 {
        let y = i as f32 * 3.0;
        mesh.push(Triangle::new(
            Vec3::new(0.0, y, 0.0),
            Vec3::new(1.0, y, 0.0),
            Vec3::new(0.0, y, 1.0),
        ));
    }
    let tree = KdTree::build(&mesh, true);
    for node in &tree.nodes {
        if !node.is_leaf() {
            assert_ne!(node.axis(), 1);
        }
    }
}

#[test]
fn terrain_tree_covers_the_block_once() {
    let tree = TerrainKdTree::build(30, 0.2);
    assert!(!tree.nodes[0].is_leaf());
    // 30 cells along z, 30 along x: equal extents split along x at cell 15.
    assert_eq!(tree.nodes[0].axis(), 0);
    assert_eq!(tree.nodes[0].value, 0.0);
    assert_eq!(tree.triangles.len(), 30 * 30 * 2);
    let mut seen = vec![0u8; 30 * 30];
    for node in &tree.nodes {
        if !node.is_leaf() {
            continue;
        }
        // A leaf lists two triangles per cell, contiguous in row-major
        // order; every leaf covers at most 3 × 3 cells.
        assert!(node.triangle_count() <= 18, "{node:?}");
        let start = node.triangle_start();
        for &tri in &tree.triangles[start..start + node.triangle_count() as usize] {
            let cell = (tri / 2) as usize;
            assert!(cell < 900);
            if tri % 2 == 0 {
                seen[cell] += 1;
            }
        }
    }
    assert!(seen.iter().all(|&c| c == 1), "{seen:?}");
}

#[test]
fn terrain_tree_translates_only_split_planes() {
    let tree = TerrainKdTree::build(30, 0.2);
    let moved = tree.translated(Vec3::new(6.0, 0.0, 6.0));
    assert_eq!(moved.nodes[0].axis(), tree.nodes[0].axis());
    assert_eq!(moved.nodes[0].value, tree.nodes[0].value + 6.0);
    assert_eq!(moved.triangles, tree.triangles);
    assert_eq!(moved.min, tree.min + Vec3::new(6.0, 0.0, 6.0));
    let mut attached = moved.clone();
    attached.attach(Vec3::splat(-24.0), Vec3::splat(-18.0));
    assert_eq!(attached.min, Vec3::splat(-24.0));
    let (x, z) = terrain_cell(Vec3::new(0.21, 0.0, 0.0), 0.2);
    assert_eq!((x, z), (1, 0));
}

#[test]
fn point_triangle_distance_regions() {
    let t = Triangle::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    assert_eq!(point_triangle_distance(Vec3::new(0.25, 0.0, 0.25), t), 0.0);
    assert_eq!(point_triangle_distance(Vec3::new(0.0, 2.0, 0.0), t), 2.0);
    assert_eq!(point_triangle_distance(Vec3::new(2.0, 0.0, 0.0), t), 1.0);
}
