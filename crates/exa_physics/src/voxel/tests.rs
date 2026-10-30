use super::*;

/// A flat, upward quad from (x0, z0) to (x1, z1) at height `y`, as two
/// triangles wound to face up.
fn quad(x0: f32, z0: f32, x1: f32, z1: f32, y: f32) -> [[Vec3; 3]; 2] {
    [
        [
            Vec3::new(x0, y, z0),
            Vec3::new(x0, y, z1),
            Vec3::new(x1, y, z1),
        ],
        [
            Vec3::new(x0, y, z0),
            Vec3::new(x1, y, z1),
            Vec3::new(x1, y, z0),
        ],
    ]
}

#[test]
fn a_map_is_made_at_the_originals_resolution() {
    let map = VoxelMap::new(0x2A);
    // 0.02 at single precision, the original's constructor value.
    assert_eq!(map.resolution.to_bits(), 0x3CA3_D70A);
    assert_eq!(map.resolution, 0.019999999552965164);
    assert_eq!(map.scale, 1.0);
    assert_eq!(map.tag, 0x2A);
    assert_eq!(map.counts, [0; 3]);
    assert!(map.cells().is_empty());
    assert!(map.fill().is_empty());
    assert!(map.spheres().is_empty());
}

#[test]
fn a_map_borders_its_cells_and_runs_z_fastest() {
    let mut map = VoxelMap::new(1);
    assert!(map.resize(
        Vec3::new(10.0, 20.0, 30.0),
        Vec3::new(4.0, 6.0, 8.0),
        [2, 3, 4]
    ));
    // Two inside cells in X become four with the border: 4 × 5 × 6.
    assert_eq!(map.dims(), [4, 5, 6]);
    assert_eq!(map.cells().len(), 120);
    // The box is 8 × 12 × 16 over 2 × 3 × 4 cells: 4 cm cells.
    assert_eq!(map.step(), Vec3::splat(4.0));
    // Z fastest, then Y, then X.
    assert_eq!(map.index([1, 2, 3]), Some(3 + 6 * 2 + 6 * 5));
    assert_eq!(map.index([4, 0, 0]), None);
    // Cell 1 is the first inside the box: its centre is half a cell in from
    // the low corner (6, 14, 22); the border cell 0 sits half a cell out.
    assert_eq!(map.cell_centre([1, 1, 1]), Some(Vec3::new(8.0, 16.0, 24.0)));
    assert_eq!(map.cell_centre([0, 0, 0]), Some(Vec3::new(4.0, 12.0, 20.0)));
    assert_eq!(
        map.cell_centre([2, 3, 4]),
        Some(Vec3::new(12.0, 24.0, 36.0))
    );
    assert!(!map.set_cell([4, 0, 0], 1));
    assert!(map.set_cell([1, 2, 3], 9));
    assert_eq!(map.cell([1, 2, 3]), Some(9));
    assert_eq!(map.cells()[3 + 6 * (2 + 5)], 9);
    // The cells start empty.
    assert_eq!(map.cells().iter().filter(|&&c| c != 0).count(), 1);
}

#[test]
fn the_array_release_keeps_the_spheres() {
    let mut map = VoxelMap::new(3);
    map.resize(Vec3::ZERO, Vec3::splat(1.0), [2, 2, 2]);
    map.set_cell([1, 1, 1], 1);
    map.set_fill(vec![[8, 0]; 64]);
    map.set_spheres(vec![Sphere {
        centre: Vec3::X,
        radius: 0.25,
    }]);
    map.release();
    // The release frees the cell words and the fill bytes; the destructor
    // frees the spheres after it.
    assert!(map.cells().is_empty());
    assert!(map.fill().is_empty());
    assert_eq!(map.spheres().len(), 1);
    // The box stays as it was; only the arrays go.
    assert_eq!(map.counts, [2, 2, 2]);
    assert_eq!(map.cell([1, 1, 1]), None);
}

#[test]
fn a_map_refuses_a_grid_that_does_not_fit() {
    let mut map = VoxelMap::new(0);
    assert!(!map.resize(Vec3::ZERO, Vec3::ONE, [usize::MAX, 2, 2]));
    assert_eq!(map.counts, [0; 3]);
    assert!(map.cells().is_empty());
}

#[test]
fn the_stored_plane_faces_along_the_first_edge_cross_the_second() {
    // (b − a) × (c − a) for a = 0, b = +Z, c = +X is +Y.
    let plane = stored_plane([Vec3::ZERO, Vec3::Z * 2.0, Vec3::X * 2.0]);
    assert_eq!(plane, [0.0, 1.0, 0.0, 0.0]);
    // The constant is the first corner along the normal.
    let raised = stored_plane([
        Vec3::Y * 5.0,
        Vec3::new(0.0, 5.0, 2.0),
        Vec3::new(2.0, 5.0, 0.0),
    ]);
    assert_eq!(raised, [0.0, 1.0, 0.0, 5.0]);
    // A degenerate face has no normal.
    assert_eq!(stored_plane([Vec3::ZERO, Vec3::X, Vec3::X * 2.0]), [0.0; 4]);
}

#[test]
fn the_plane_distance_sums_x_and_y_first() {
    // (1e8 + 1) − 1e8 is 0 in single precision; (−1e8 + 1e8) + 1 is 1.
    let plane = [1.0, 1.0, -1.0, 0.0];
    assert_eq!(plane_distance(&plane, [1e8, 1.0, 1e8]), 0.0);
}

#[test]
fn the_ray_finds_the_highest_upward_face() {
    let faces: Vec<SurfaceTriangle> = quad(-300.0, -300.0, 300.0, 300.0, 20.0)
        .into_iter()
        .chain(quad(-300.0, -300.0, 300.0, 300.0, 10.0))
        .filter_map(SurfaceTriangle::new)
        .collect();
    // From y = 30 down: the face at 20 is 10 away.
    let hit = surface_hit(&faces, Vec3::new(5.0, 30.0, 5.0), Vec3::NEG_Y, 50.0);
    assert_eq!(hit, Some(10.0));
    // Out of the triangles' reach.
    assert_eq!(
        surface_hit(&faces, Vec3::new(400.0, 30.0, 5.0), Vec3::NEG_Y, 50.0),
        None
    );
    // A reach too short for the nearest face.
    assert_eq!(
        surface_hit(&faces, Vec3::new(5.0, 30.0, 5.0), Vec3::NEG_Y, 5.0),
        None
    );
}

#[test]
fn a_triangle_facing_down_is_not_surface() {
    // The same quad wound the other way.
    let [a, b, c] = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 10.0),
    ];
    assert_eq!(SurfaceTriangle::new([a, b, c]), None);
    // A steep face still counts as long as its normal turns up.
    assert!(
        SurfaceTriangle::new([
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 1.0),
            Vec3::new(10.0, 0.0, 0.0),
        ])
        .is_some()
    );
    // Level with y: no surface.
    assert_eq!(
        SurfaceTriangle::new([
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ]),
        None
    );
    // Degenerate.
    assert_eq!(
        SurfaceTriangle::new([Vec3::ZERO, Vec3::X, Vec3::X * 2.0]),
        None
    );
}

#[test]
fn the_ray_keeps_the_triangles_edges_inside() {
    // A face in the plane y = x, over x + z ≤ 10: (0,0,0), (0,0,10), (10,10,0).
    let face = SurfaceTriangle::new([
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(10.0, 10.0, 0.0),
    ])
    .expect("faces up");
    let faces = [face];
    // (2, 20, 2) meets the plane at y = 2, 18 below.
    let hit = surface_hit(&faces, Vec3::new(2.0, 20.0, 2.0), Vec3::NEG_Y, 50.0);
    assert!((hit.expect("inside") - 18.0).abs() < 1e-4);
    // (5, 20, 5) sits on the last edge, x + z = 10: the edge is inside.
    let hit = surface_hit(&faces, Vec3::new(5.0, 20.0, 5.0), Vec3::NEG_Y, 50.0);
    assert!((hit.expect("on the edge") - 15.0).abs() < 1e-4);
    // (8, 20, 8) lies past it.
    assert_eq!(
        surface_hit(&faces, Vec3::new(8.0, 20.0, 8.0), Vec3::NEG_Y, 50.0),
        None
    );
}

#[test]
fn a_flat_floor_marks_the_band_under_it() {
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::new(300.0, 0.0, 300.0),
    };
    let triangles = quad(-300.0, -300.0, 300.0, 300.0, 0.0);
    let points = floor_points(bounds, &triangles, 20.0);
    // The bounds grow to [−300, 300] × [−40, 0] × [−300, 300], at 10 cm
    // cells: 60 × 4 × 60. Every 2 × 2 × 2 block has its four columns marked
    // at fine y = 3, the band under the surface, so 30 × 30 points.
    assert_eq!(points.len(), 30 * 30);
    // The points come in Z, then Y, then X order, at 20 cm steps.
    assert_eq!(points[0], Vec3::new(-290.0, -10.0, -290.0));
    assert_eq!(points[29], Vec3::new(290.0, -10.0, -290.0));
    assert_eq!(points[30], Vec3::new(-290.0, -10.0, -270.0));
    assert_eq!(points[899], Vec3::new(290.0, -10.0, 290.0));
}

#[test]
fn a_hole_leaves_the_blocks_over_it_empty() {
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::new(300.0, 0.0, 300.0),
    };
    // A 6 m floor with a 2 m square hole in the middle.
    let mut triangles = Vec::new();
    triangles.extend(quad(-300.0, -300.0, -100.0, 300.0, 0.0));
    triangles.extend(quad(100.0, -300.0, 300.0, 300.0, 0.0));
    triangles.extend(quad(-100.0, -300.0, 100.0, -100.0, 0.0));
    triangles.extend(quad(-100.0, 100.0, 100.0, 300.0, 0.0));
    let points = floor_points(bounds, &triangles, 20.0);
    // Ten 20 cm blocks across the hole on each axis are lost: 900 − 100.
    assert_eq!(points.len(), 800);
    assert!(
        points
            .iter()
            .all(|p| p.x.abs() >= 100.0 || p.z.abs() >= 100.0),
        "a point stands over the hole"
    );
}

#[test]
fn the_floor_bounds_snap_outward() {
    // A mesh from x = 50 to 150: the edge rule picks 0 for the low edge and
    // 150 for the high one; neither widens, and the high one snaps out to
    // 160 (round(7.5 + 0.1) = 8).
    let bounds = Bounds {
        centre: Vec3::new(100.0, 0.0, 0.0),
        half_extents: Vec3::new(50.0, 0.0, 300.0),
    };
    let triangles = quad(50.0, -300.0, 150.0, 300.0, 0.0);
    let points = floor_points(bounds, &triangles, 20.0);
    // X columns are at 5 … 155; the blocks over 45 … 55 and 145 … 155 have
    // one column outside the mesh, so the points run 70 … 130: 4 × 30.
    assert_eq!(points.len(), 4 * 30);
    assert_eq!(points[0], Vec3::new(70.0, -10.0, -290.0));
    assert_eq!(points[3], Vec3::new(130.0, -10.0, -290.0));
}

#[test]
fn a_deep_floor_keeps_its_y_lattice() {
    // The mesh is a floor 50 cm down.
    let bounds = Bounds {
        centre: Vec3::new(0.0, -50.0, 0.0),
        half_extents: Vec3::new(300.0, 0.0, 300.0),
    };
    // The low edge is snapped to −60 and then lowered 20 more, so the grid
    // runs from −80 to 0 in 10 cm cells.
    let triangles = quad(-300.0, -300.0, 300.0, 300.0, -50.0);
    let points = floor_points(bounds, &triangles, 20.0);
    assert_eq!(points.len(), 900);
    // The surface is 60 below the ray start, so the mark is fine cell 2;
    // the 20 cm block over it spans −60 … −40 and its point sits at −50.
    assert!(points.iter().all(|p| p.y == -50.0));
}

#[test]
fn a_floor_needs_a_positive_parameter_and_an_upward_face() {
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::new(300.0, 0.0, 300.0),
    };
    let triangles = quad(-300.0, -300.0, 300.0, 300.0, 0.0);
    assert!(floor_points(bounds, &triangles, 0.0).is_empty());
    // Facing down: nothing is hit and nothing is marked.
    let down = [
        [
            Vec3::new(-300.0, 0.0, -300.0),
            Vec3::new(300.0, 0.0, 300.0),
            Vec3::new(-300.0, 0.0, 300.0),
        ],
        [
            Vec3::new(-300.0, 0.0, -300.0),
            Vec3::new(300.0, 0.0, -300.0),
            Vec3::new(300.0, 0.0, 300.0),
        ],
    ];
    assert!(floor_points(bounds, &down, 20.0).is_empty());
    // No triangles at all.
    assert!(floor_points(bounds, &[], 20.0).is_empty());
}

#[test]
fn the_surface_grid_counts_are_odd_and_round_ties_even() {
    let dims = |half: f32| {
        surface_dims(
            Bounds {
                centre: Vec3::ZERO,
                half_extents: Vec3::new(half, half, half),
            },
            20.0,
        )
    };
    // round(10 · half / 20) + 3, forced odd.
    assert_eq!(dims(10.0), [9; 3]); // 5 + 3
    assert_eq!(dims(11.0), [9; 3]); // 5.5 rounds to 6, + 3
    assert_eq!(dims(12.0), [9; 3]); // 6 + 3
    assert_eq!(dims(13.0), [9; 3]); // 6.5 rounds to 6, + 3
    assert_eq!(dims(14.0), [11; 3]); // 7 + 3, forced odd
    assert_eq!(dims(15.0), [11; 3]); // 7.5 rounds to 8, + 3
}

#[test]
fn a_plane_of_cells_covers_into_four_points() {
    // A slab through y = 0: of the 9 × 9 × 9 cells, only the middle layer
    // of 7 × 7 is on.
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.0),
    };
    let plane = |p: Vec3, r: f32| p.y.abs() <= r;
    let points = surface_points(bounds, 20.0, plane, SurfaceCover::Greedy);
    // The greedy cover takes four points, a quarter in from each corner.
    assert_eq!(
        points,
        vec![
            Vec3::new(-4.0, 0.0, -4.0),
            Vec3::new(4.0, 0.0, -4.0),
            Vec3::new(-4.0, 0.0, 4.0),
            Vec3::new(4.0, 0.0, 4.0),
        ]
    );
}

#[test]
fn every_cell_gets_its_own_point() {
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.0),
    };
    let plane = |p: Vec3, r: f32| p.y.abs() <= r;
    let points = surface_points(bounds, 20.0, plane, SurfaceCover::EveryCell);
    assert_eq!(points.len(), 7 * 7);
    // Cells 1 … 7 at 4 cm steps about the centre; scan order is Z, Y, X.
    assert_eq!(points[0], Vec3::new(-12.0, 0.0, -12.0));
    assert_eq!(points[6], Vec3::new(12.0, 0.0, -12.0));
    assert_eq!(points[7], Vec3::new(-12.0, 0.0, -8.0));
    assert_eq!(points[48], Vec3::new(12.0, 0.0, 12.0));
}

#[test]
fn near_cells_next_to_the_surface_are_cleared() {
    // A slab through y = 2.5: the cells at y = 0 are within the larger
    // sphere (2.83) but not the contact one (2), so they are near; the row
    // at y = 4 touches. The near row is cleared, leaving one on layer.
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.0),
    };
    let slab = |p: Vec3, r: f32| (p.y - 2.5).abs() <= r;
    let points = surface_points(bounds, 20.0, slab, SurfaceCover::Greedy);
    assert_eq!(points.len(), 4);
    assert!(points.iter().all(|p| p.y == 4.0));
    // Without the clearing there would be two layers of 7 × 7 cells.
    let all = surface_points(bounds, 20.0, slab, SurfaceCover::EveryCell);
    assert_eq!(all.len(), 7 * 7);
    assert!(all.iter().all(|p| p.y == 4.0));
}

#[test]
fn the_cover_prefers_the_first_of_equal_centres() {
    // A solid 15 × 15 × 15 block of cells: every fully inside 5 × 5 × 5
    // window scores the same, so the first one in Z, Y, X order wins.
    let mut grid = vec![SURFACE_ON; 15 * 15 * 15];
    let dims = [15, 15, 15];
    let list = clean_and_list(&mut grid, dims);
    assert_eq!(list.len(), 13 * 13 * 13);
    let points = greedy_points(&mut grid, dims, Vec3::ZERO, 4.0, &list);
    assert_eq!(points.len(), 27);
    // The first cell of the list is (1, 1, 1); its best window starts at
    // (2, 2, 2) and the other full windows tie with it.
    assert_eq!(points[0], Vec3::new(-20.0, -20.0, -20.0));
    assert_eq!(points[1], Vec3::new(0.0, -20.0, -20.0));
    assert_eq!(points[8], Vec3::new(20.0, 20.0, -20.0));
    assert_eq!(points[9], Vec3::new(-20.0, -20.0, 0.0));
    assert_eq!(points[26], Vec3::new(20.0, 20.0, 20.0));
}

#[test]
fn a_sparse_grid_gets_no_point() {
    // The cover starts from −1000; a window holding one cell of 125 scores
    // 40 − 4 · 300, below it, so nothing is emitted and nothing consumed.
    let mut grid = vec![0u8; 9 * 9 * 9];
    grid[3 + 3 * 9 + 3 * 81] = SURFACE_ON;
    let dims = [9, 9, 9];
    let list = clean_and_list(&mut grid, dims);
    assert_eq!(list.len(), 1);
    assert!(greedy_points(&mut grid, dims, Vec3::ZERO, 4.0, &list).is_empty());
}

#[test]
fn the_pass_lists_sparse_cells_first() {
    // The middle layer of a 9 × 9 × 9 grid: corners see 4 cells, edges 6,
    // the inside 9.
    let mut grid = vec![0u8; 9 * 9 * 9];
    for z in 1..8 {
        for x in 1..8 {
            grid[x + 4 * 9 + z * 81] = SURFACE_ON;
        }
    }
    let list = clean_and_list(&mut grid, [9, 9, 9]);
    assert_eq!(list.len(), 49);
    let cells: Vec<[u16; 3]> = list.iter().map(|e| e.cell).collect();
    assert_eq!(cells[0], [1, 4, 1]);
    assert_eq!(cells[1], [7, 4, 1]);
    assert_eq!(cells[2], [1, 4, 7]);
    assert_eq!(cells[3], [7, 4, 7]);
    assert_eq!(list[0].count, 4);
    assert_eq!(list[4].count, 6);
    assert_eq!(list[24].count, 9);
    assert_eq!(list[48].cell, [6, 4, 6]);
    assert_eq!(list[48].count, 9);
}

#[test]
fn near_cells_without_an_on_neighbour_become_on() {
    let mut grid = vec![0u8; 9 * 9 * 9];
    // One near cell alone, one near cell beside an on cell.
    grid[2 + 2 * 9 + 2 * 81] = SURFACE_NEAR_CELL;
    grid[2 + 5 * 9 + 5 * 81] = SURFACE_NEAR_CELL;
    grid[3 + 5 * 9 + 5 * 81] = SURFACE_ON;
    let list = clean_and_list(&mut grid, [9, 9, 9]);
    assert_eq!(grid[2 + 2 * 9 + 2 * 81], SURFACE_ON);
    assert_eq!(grid[2 + 5 * 9 + 5 * 81], 0);
    assert!(list.iter().any(|e| e.cell == [2, 2, 2]));
    assert!(!list.iter().any(|e| e.cell == [2, 5, 5]));
}

#[test]
fn shared_points_are_the_masters() {
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.0),
    };
    let plane = |p: Vec3, r: f32| p.y.abs() <= r;
    let first = shared_surface_points(bounds, 20.0, plane, SurfaceCover::Greedy);
    let second = first.clone();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.len(), 4);
    // The same generation as the unshared call.
    assert_eq!(
        *first,
        surface_points(bounds, 20.0, plane, SurfaceCover::Greedy)
    );
}

#[test]
fn a_sphere_touches_a_mesh_through_its_closest_point() {
    // The triangle (0,0,0), (10,0,0), (0,0,10) in the ground plane.
    let t = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 10.0),
    ];
    let mesh = [t];
    // Above the inside, a centimetre up: a 1 cm sphere just reaches it.
    assert!(mesh_touches(&mesh, Vec3::new(2.0, 0.01, 2.0), 0.01));
    assert!(!mesh_touches(&mesh, Vec3::new(2.0, 0.02, 2.0), 0.01));
    // Past the hypotenuse, the closest point is on the edge.
    assert!(!mesh_touches(&mesh, Vec3::new(20.0, 0.0, 0.0), 5.0));
    assert!(mesh_touches(&mesh, Vec3::new(20.0, 0.0, 0.0), 10.0));
    // Past a corner.
    assert!(!mesh_touches(&mesh, Vec3::new(-3.0, 4.0, 0.0), 4.9));
    assert!(mesh_touches(&mesh, Vec3::new(-3.0, 4.0, 0.0), 5.0));
    // No mesh, no touch.
    assert!(!mesh_touches(&[], Vec3::ZERO, 100.0));
}

#[test]
fn a_mesh_query_gives_the_surface_generator_its_points() {
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.0),
    };
    // A ground plane at y = 0.
    let mesh = [
        [
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::new(-20.0, 0.0, 20.0),
            Vec3::new(20.0, 0.0, 20.0),
        ],
        [
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::new(20.0, 0.0, 20.0),
            Vec3::new(20.0, 0.0, -20.0),
        ],
    ];
    let points = surface_points(
        bounds,
        20.0,
        |p, r| mesh_touches(&mesh, p, r),
        SurfaceCover::Greedy,
    );
    // The same four points the synthetic slab gave.
    assert_eq!(
        points,
        vec![
            Vec3::new(-4.0, 0.0, -4.0),
            Vec3::new(4.0, 0.0, -4.0),
            Vec3::new(-4.0, 0.0, 4.0),
            Vec3::new(4.0, 0.0, 4.0),
        ]
    );
}

#[test]
fn a_generator_without_a_surface_gives_nothing() {
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.0),
    };
    let nothing = |_: Vec3, _: f32| false;
    assert!(surface_points(bounds, 20.0, nothing, SurfaceCover::Greedy).is_empty());
    // No step, no cells.
    assert!(surface_points(bounds, 0.0, nothing, SurfaceCover::Greedy).is_empty());
    // No size, no sampling.
    let flat = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::ZERO,
    };
    assert!(surface_points(flat, 20.0, nothing, SurfaceCover::EveryCell).is_empty());
}

#[test]
fn an_edge_snaps_in_whole_steps() {
    // round(−15 − 0.1) · 20 and round(15 + 0.1) · 20.
    assert_eq!(snap_edge(-300.0, 0.1), -300.0);
    assert_eq!(snap_edge(300.0, -0.1), 300.0);
    // 7.5 + 0.1 rounds up, 7.5 − 0.1 down.
    assert_eq!(snap_edge(150.0, -0.1), 160.0);
    assert_eq!(snap_edge(150.0, 0.1), 140.0);
    // −2.5 − 0.1 rounds to −3.
    assert_eq!(snap_edge(-50.0, 0.1), -60.0);
}

#[test]
fn an_odd_row_count_drifts_the_downsample_back() {
    // 2 × 3 × 4 fine cells: one block per slice pair. The original steps two
    // rows per block row (one block row here) and then one slice, so the
    // second block slice starts at row 2 of slice 1 (index 10), not at
    // slice 2 (index 12).
    let dims = [2, 3, 4];
    let mut marks = vec![0u8; 24];
    for i in [10, 11, 12, 13] {
        marks[i] = 0xFF;
    }
    let points = downsample(&marks, dims, [0.0; 3]);
    assert_eq!(points, vec![Vec3::new(10.0, 10.0, 30.0)]);
    // An even row count reads each block where it is.
    let dims = [2, 2, 4];
    let mut marks = vec![0u8; 16];
    for i in [8, 9, 10, 11] {
        marks[i] = 0xFF;
    }
    assert_eq!(
        downsample(&marks, dims, [0.0; 3]),
        vec![Vec3::new(10.0, 10.0, 30.0)]
    );
}

#[test]
fn the_surface_step_and_near_radius_are_doubles() {
    // 13 · 0.2 is 2.6000001 in single precision, 2.6 through a double.
    assert_eq!(surface_step(13.0), 2.6);
    assert_ne!(13.0f32 * 0.2f32, 2.6);
    assert_eq!(surface_step(20.0), 4.0);
    // 6.6 · √2 / 2 is 4.666905 through doubles, 4.6669044 in singles.
    let step = surface_step(33.0);
    assert_eq!(near_radius(step), 4.666905);
    assert_ne!(step * 1.4142135f32 * 0.5, 4.666905);
    assert_eq!(near_radius(4.0), 2.828427);
}

#[test]
fn the_columns_meet_the_same_faces_as_the_whole_list() {
    // Flat, tilted and steep faces over a 10 × 10 grid of 10 cm columns.
    let mut triangles = vec![
        [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 5.0, 60.0),
            Vec3::new(60.0, 10.0, 0.0),
        ],
        [
            Vec3::new(20.0, 30.0, 20.0),
            Vec3::new(20.0, 30.0, 45.0),
            Vec3::new(45.0, 30.0, 45.0),
        ],
        // Steep: tested on y and z.
        [
            Vec3::new(44.0, 0.0, 10.0),
            Vec3::new(44.0, 40.0, 90.0),
            Vec3::new(47.0, 0.0, 90.0),
        ],
    ];
    triangles.extend(quad(55.0, 55.0, 100.0, 100.0, 20.0));
    let faces: Vec<SurfaceTriangle> = triangles
        .iter()
        .filter_map(|&t| SurfaceTriangle::new(t))
        .collect();
    assert_eq!(faces.len(), 5);
    assert!(faces[2].footprint.is_none());
    let columns = face_columns(&faces, 10, 10, [0.0, 0.0], 10.0).expect("fits");
    let mut hits = 0;
    for z in 0..10 {
        for x in 0..10 {
            let origin = Vec3::new(x as f32 * 10.0 + 5.0, 60.0, z as f32 * 10.0 + 5.0);
            let all = surface_hit(&faces, origin, Vec3::NEG_Y, 80.0);
            let some = hit_among(
                columns[x + z * 10].iter().map(|&i| &faces[i as usize]),
                origin,
                Vec3::NEG_Y,
                80.0,
            );
            assert_eq!(all, some, "column {x}, {z}");
            hits += all.is_some() as usize;
        }
    }
    assert!(hits > 40);
}

#[test]
fn the_floor_sets_flagged_children_get_generated_points() {
    // The original generates surface points for a floor-set child when
    // bit 0x10000 or bit 0x8 of its second flag word is set.
    assert!(floor_set_node_generates(0x8));
    assert!(floor_set_node_generates(0x10000));
    assert!(floor_set_node_generates(0x10008));
    assert!(floor_set_node_generates(0xffff_ffff));
    assert!(!floor_set_node_generates(0));
    assert!(!floor_set_node_generates(0x10));
    assert!(!floor_set_node_generates(0x1000));
    assert!(!floor_set_node_generates(0x2_0000));
    assert_eq!(GENERATED_POINT_FLAGS2, 0x10008);
}

#[test]
fn generated_points_are_the_centimetre_run_scaled_back() {
    // A metre-space mesh is the original's centimetre mesh divided by 100;
    // the generator runs in centimetres at its runtime parameter 20.
    let triangles = quad(-0.5, -0.5, 0.5, 0.5, 0.0);
    let bounds = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::new(0.5, 0.0, 0.5),
    };
    let points = generated_surface_points(bounds, &triangles, SurfaceCover::Greedy);
    assert!(!points.is_empty());
    let cm_triangles: Vec<[Vec3; 3]> = triangles.iter().map(|t| t.map(|p| p * 100.0)).collect();
    let expected: Vec<Vec3> = surface_points(
        Bounds {
            centre: bounds.centre * 100.0,
            half_extents: bounds.half_extents * 100.0,
        },
        GENERATED_POINT_PARAMETER,
        |point, radius| mesh_touches(&cm_triangles, point, radius),
        SurfaceCover::Greedy,
    )
    .into_iter()
    .map(|p| p / 100.0)
    .collect();
    assert_eq!(points, expected);
    // Every point stays inside the mesh's own bounds.
    for p in &points {
        assert!(p.abs().cmple(bounds.half_extents + Vec3::splat(0.01)).all());
    }
}
