use super::*;

/// The six sides of an axis-aligned box as outward-wound triangles, each
/// side split along the diagonal from its (−, −) corner to its (+, +) one.
fn box_triangles(centre: Vec3, half: Vec3) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for axis in 0..3 {
        let (b, c) = ((axis + 1) % 3, (axis + 2) % 3);
        for sign in [-1.0f32, 1.0] {
            let corner = |u: f32, v: f32| {
                let mut p = centre;
                p[axis] += sign * half[axis];
                p[b] += u * half[b];
                p[c] += v * half[c];
                p
            };
            let mut quad = [
                corner(-1.0, -1.0),
                corner(1.0, -1.0),
                corner(1.0, 1.0),
                corner(-1.0, 1.0),
            ];
            if sign < 0.0 {
                quad.reverse();
            }
            out.push([quad[0], quad[1], quad[2]]);
            out.push([quad[0], quad[2], quad[3]]);
        }
    }
    out
}

/// A map of `counts` cells over the box `half` around the origin with
/// every inner cell filled.
fn solid_map(half: Vec3, counts: [usize; 3]) -> VoxelMap {
    let mut map = VoxelMap::new(BUILDER_MAP_TAG);
    assert!(map.resize(Vec3::ZERO, half, counts));
    for i in 1..=counts[0] {
        for j in 1..=counts[1] {
            for k in 1..=counts[2] {
                map.set_cell([i, j, k], 1);
            }
        }
    }
    map
}

#[test]
fn the_entries_sort_by_key() {
    let mut entries = [(3.0, 0), (1.0, 1), (2.0, 2), (1.0, 3)];
    sort_by_key(&mut entries);
    let keys: Vec<f32> = entries.iter().map(|(key, _)| *key).collect();
    assert_eq!(keys, vec![1.0, 1.0, 2.0, 3.0]);
    // Seven equal keys come out in the quicksort's own order: the first
    // pass reverses them, then each side swaps its ends.
    let mut equal: Vec<(f32, u32)> = (0..7).map(|i| (3.0, i)).collect();
    sort_by_key(&mut equal);
    let order: Vec<u32> = equal.iter().map(|e| e.1).collect();
    assert_eq!(order, vec![4, 5, 6, 3, 0, 1, 2]);
}

/// One filled cell in a 1-cell grid: the centre is the box's and the mass
/// the cell's volume.
#[test]
fn a_unit_volume_summarises_its_centre_and_mass() {
    let mut map = VoxelMap::new(0);
    assert!(map.resize(Vec3::ZERO, Vec3::new(2.0, 4.0, 6.0), [1, 1, 1]));
    map.set_cell([1, 1, 1], 1);
    let summary = summarise(&map);
    assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
    assert_eq!(summary.mass, 8.0 * 48.0);
}

/// A grid with an axis of no cells has the fallback mass; one with no
/// filled cell weighs nothing and has no centre (0/0).
#[test]
fn an_empty_volume_weighs_as_the_original() {
    let map = VoxelMap::new(0);
    let summary = summarise(&map);
    assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
    assert_eq!(summary.mass, EMPTY_MASS);
    let mut map = VoxelMap::new(0);
    assert!(map.resize(Vec3::ZERO, Vec3::ONE, [2, 2, 2]));
    let summary = summarise(&map);
    assert_eq!(summary.mass, 0.0);
    assert!(summary.centre.iter().all(|c| c.is_nan()));
}

/// Two cells of a 2 × 2 × 2 grid at (1, 1, 1) and (1, 1, 2): the mass
/// multiplies as count · (z · (y · x)), and the centre is the original's
/// expression, half the cells' offset: z sums (1 − ½) + (2 − ½) = 2, so
/// (2/2/2 − ½)·1 = 0, while x and y give (½/2 − ½) = −¼.
#[test]
fn the_mass_and_centre_follow_the_originals_arithmetic() {
    let mut map = VoxelMap::new(0);
    assert!(map.resize(Vec3::ZERO, Vec3::ONE, [2, 2, 2]));
    map.set_cell([1, 1, 1], 1);
    map.set_cell([1, 1, 2], 1);
    let summary = summarise(&map);
    assert_eq!(summary.mass, 2.0);
    assert_eq!(summary.centre, [-0.25, -0.25, 0.0]);
}

/// The grid's counts: the longest axis rounds to even (CVTSS2SI), is made
/// even, at least six cells and at most 96; the others round their
/// proportions up (the original's round helper sets the rounding mode
/// toward +∞), made even when asked.
#[test]
fn the_grid_counts_round_up_the_short_axes() {
    let counts = |half: Vec3, options: VolumeOptions| {
        let mesh = Mesh::with_bounds(
            &box_triangles(Vec3::ZERO, half),
            Bounds {
                centre: Vec3::ZERO,
                half_extents: half,
            },
        );
        let mut map = VoxelMap::new(BUILDER_MAP_TAG);
        assert!(fill_near_surface(&mut map, &mesh, &options));
        map.counts
    };
    let options = VolumeOptions::default();
    // 35·0.3 = 10.5 rounds to even 10; 10·10/35 = 2.86 rounds up to 3,
    // made even to 4.
    assert_eq!(counts(Vec3::new(35.0, 10.0, 10.0), options), [10, 4, 4]);
    // 15·0.3 = 4.5 rounds to 4, raised to six; 6·10/15 = 4 exactly.
    assert_eq!(counts(Vec3::new(15.0, 10.0, 10.0), options), [6, 4, 4]);
    // Without the even rule: 20·0.3 = 6; 6·7/20 = 2.1 → 3; 6·1/20 = 0.3 → 1.
    let odd = VolumeOptions {
        even_axes: false,
        ..options
    };
    assert_eq!(counts(Vec3::new(20.0, 7.0, 1.0), odd), [6, 3, 1]);
    assert_eq!(counts(Vec3::new(20.0, 7.0, 1.0), options), [6, 4, 2]);
    // A tie for the longest axis keeps the first; 400·0.3 = 120 → 96.
    assert_eq!(counts(Vec3::new(400.0, 400.0, 4.0), options), [96, 96, 2]);
    // A longest half extent under 1 cm makes no grid.
    let mesh = Mesh::with_bounds(
        &[],
        Bounds {
            centre: Vec3::ZERO,
            half_extents: Vec3::splat(0.9),
        },
    );
    let mut map = VoxelMap::new(0);
    assert!(!fill_near_surface(&mut map, &mesh, &options));
    assert_eq!(map.counts, [0; 3]);
}

/// The builders' resolution: 0.6 cells per cm under a 10 cm smallest half
/// extent, 0.3 from it, and one cell per cm (eight at least) with the
/// quality flag.
#[test]
fn the_builders_choose_the_resolution_by_the_smallest_half_extent() {
    let bounds = |h: Vec3| Bounds {
        centre: Vec3::ZERO,
        half_extents: h,
    };
    let fine = VolumeOptions::for_bounds(&bounds(Vec3::new(30.0, 10.0, 12.0)), false);
    assert_eq!(
        (fine.resolution, fine.min_depth),
        (FINE_RESOLUTION, MIN_DEPTH)
    );
    let coarse = VolumeOptions::for_bounds(&bounds(Vec3::new(30.0, 9.99, 12.0)), false);
    assert_eq!(coarse.resolution, COARSE_RESOLUTION);
    let precise = VolumeOptions::for_bounds(&bounds(Vec3::new(30.0, 2.0, 2.0)), true);
    assert_eq!(
        (precise.resolution, precise.min_depth),
        (PRECISE_RESOLUTION, PRECISE_MIN_DEPTH)
    );
    assert_eq!((fine.min_corners, fine.min_free_corners), (1, 0));
    assert!(fine.even_axes && !fine.wide_queries);
}

/// A face's sphere test: within the plane's reach, then strictly inside
/// the squared radius less the epsilon. A point one cm past a corner is
/// reached by a radius one ulp over 1, not by 1 itself.
#[test]
fn a_sphere_touches_a_face_strictly_inside_its_radius() {
    let face = Face::new([
        Vec3::ZERO,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    let past = Vec3::new(-1.0, 0.0, 0.0);
    assert_eq!(face.nearest(past, 0.0), Vec3::ZERO);
    assert!(!face.touches(past, 1.0));
    assert!(face.touches(past, f32::from_bits(1.0f32.to_bits() + 1)));
    // Over the face, 1 cm above it.
    let above = Vec3::new(0.5, 0.5, 1.0);
    assert!(face.touches(above, 1.1));
    assert!(!face.touches(above, 0.9));
    // Beyond the far edge: the nearest point is on it, (1, 1, 0).
    let beyond = Vec3::new(2.0, 2.0, 0.0);
    let d = face.nearest(beyond, 0.0) - Vec3::new(1.0, 1.0, 0.0);
    assert!(d.length() < 1e-3, "{d}");
    assert!(face.touches(beyond, 1.5));
    assert!(!face.touches(beyond, 1.3));
}

/// The ray test is one-sided: a face is hit only from behind (the ray
/// along its normal), between 1e-8 and the ray's length.
#[test]
fn a_ray_hits_a_face_only_along_its_normal() {
    // Normal +z.
    let face = Face::new([
        Vec3::ZERO,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ]);
    assert!(face.normal.z > 0.0);
    let below = Vec3::new(0.5, 0.5, -1.0);
    assert!(face.ray_hits(below, Vec3::Z, 1.5));
    assert!(!face.ray_hits(below, Vec3::Z, 0.5));
    // From above, down through it: the face points the other way.
    let above = Vec3::new(0.5, 0.5, 1.0);
    assert!(!face.ray_hits(above, -Vec3::Z, 5.0));
    // Outside the triangle.
    assert!(!face.ray_hits(Vec3::new(1.5, 1.5, -1.0), Vec3::Z, 5.0));
    // On the plane: no distance to go.
    assert!(!face.ray_hits(Vec3::new(0.5, 0.5, 0.0), Vec3::Z, 5.0));
}

/// The box helper winds every side outward, as the one-sided rays need.
#[test]
fn the_test_box_faces_outward() {
    for t in box_triangles(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)) {
        let face = Face::new(t);
        let middle = (t[0] + t[1] + t[2]) / 3.0 - Vec3::new(1.0, 2.0, 3.0);
        assert!(face.normal.dot(middle) > 0.0);
    }
}

/// The bounding-volume tree answers exactly as a test of every face does,
/// for sphere and ray queries over a mesh of many faces, including points
/// on a cell lattice where queries land on edges and at their radius.
#[test]
fn the_tree_matches_testing_every_face() {
    let centre = Vec3::new(1.0, -2.0, 3.0);
    let half = Vec3::new(10.0, 6.0, 4.0);
    let mut triangles = Vec::new();
    // Each side cut into 4 × 4 boxes' worth of strips.
    for i in 0..4 {
        let x = -half.x + half.x * 0.5 * (i as f32 + 0.5);
        triangles.extend(box_triangles(
            centre + Vec3::new(x, 0.0, 0.0),
            Vec3::new(half.x * 0.25, half.y, half.z),
        ));
    }
    let mesh = Mesh::new(&triangles);
    assert_eq!(mesh.bounds().centre, centre);
    assert_eq!(mesh.bounds().half_extents, half);
    let every_touch = |p: Vec3, r: f32| mesh.faces().iter().any(|f| f.touches(p, r));
    let every_ray = |p: Vec3, d: Vec3, l: f32| mesh.faces().iter().any(|f| f.ray_hits(p, d, l));
    let mut tested = 0;
    for i in -9..=9 {
        for j in -5..=5 {
            for k in -4..=4 {
                let p = centre + Vec3::new(i as f32 * 1.25, j as f32 * 1.25, k as f32 * 1.0);
                for r in [0.5f32, 1.0, 2.5, 6.0] {
                    assert_eq!(mesh.touches(p, r), every_touch(p, r), "touch at {p} r {r}");
                    tested += 1;
                }
                for d in [Vec3::X, -Vec3::Y, Vec3::Z, -Vec3::Z] {
                    for l in [3.0f32, 40.0] {
                        assert_eq!(
                            mesh.ray(p, d, l),
                            every_ray(p, d, l),
                            "ray at {p} along {d}"
                        );
                        tested += 1;
                    }
                }
            }
        }
    }
    assert!(tested > 10_000);
}

/// A 20 × 18.8 × 18.2 cm box fitted over a 21 cm cube's grid: 6 cells a
/// side of 3.5 cm, the outer layer within 1.75 cm of the surface, the
/// inner 4 × 4 × 4 solid by the rays, so all 216 cells weigh 216 · 3.5³ =
/// 9261 cm³ with the centre at the box's. The one candidate, (3, 3, 3), is
/// 3 cells from the nearest cell short of eight corners: a sphere of
/// 3/(1 + 0.03) cells of 3.5 cm at the box's centre comes first, then
/// corner spheres one cell across.
#[test]
fn a_box_fits_its_volume_and_a_central_sphere() {
    let half = Vec3::new(10.0, 9.4, 9.1);
    let grid = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.5),
    };
    let mesh = Mesh::with_bounds(&box_triangles(Vec3::ZERO, half), grid);
    let options = VolumeOptions::for_bounds(&grid, false);
    assert_eq!(options.resolution, FINE_RESOLUTION);
    let fitted = fit(&mesh, &options);
    assert_eq!(fitted.counts, [6, 6, 6]);
    assert_eq!(fitted.summary.mass, 9261.0);
    assert_eq!(fitted.summary.centre, [0.0, 0.0, 0.0]);
    assert!((fitted.summary.mobility() - 10_000.0 / 9261.0).abs() < 1e-6);
    let head = fitted.spheres[0];
    assert_eq!(head.centre, Vec3::ZERO);
    assert_eq!(head.radius, (3.0f64 / 1.03 * 3.5) as f32);
    assert!(fitted.spheres.len() > 1);
    for s in &fitted.spheres[1..] {
        assert_eq!(s.radius, 3.5);
        assert!(s.centre.abs().max_element() < 14.0, "{s:?}");
    }
}

/// The layers of the near-surface fill on the same box: the first and last
/// cell of each axis are marked by the sphere query and the rest are
/// empty until the solid fill.
#[test]
fn the_surface_fill_marks_the_outer_layer() {
    let grid = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::splat(10.5),
    };
    let mesh = Mesh::with_bounds(&box_triangles(Vec3::ZERO, Vec3::new(10.0, 9.4, 9.1)), grid);
    let mut map = VoxelMap::new(BUILDER_MAP_TAG);
    assert!(fill_near_surface(
        &mut map,
        &mesh,
        &VolumeOptions::default()
    ));
    for i in 1..=6 {
        for j in 1..=6 {
            for k in 1..=6 {
                let outer = [i, j, k].iter().any(|&c| c == 1 || c == 6);
                assert_eq!(map.cell([i, j, k]), Some(i32::from(outer)), "{i} {j} {k}");
            }
        }
    }
    assert_eq!(fill_solid(&mut map, &mesh), 216);
}

/// The redundancy pruning on a solid rod of 12 × 6 × 6 cells: seven
/// candidates of radius 3 (cells 3 to 9 along x), sorted by the quicksort
/// into the order 7, 8, 9, 6, 3, 4, 5. Cells 7 and 4 are covered twice
/// everywhere and are dropped in turn; 8, 9, 6, 3 and 5 each have a cell
/// no other survivor covers, and keep their spheres, in that order.
#[test]
fn redundant_candidates_are_pruned_in_the_sorted_order() {
    let mut map = solid_map(Vec3::new(12.0, 6.0, 6.0), [12, 6, 6]);
    mark_corners(&mut map);
    extract_spheres(&mut map, &VolumeOptions::default());
    let radius = (3.0f64 / 1.03 * 2.0) as f32;
    let candidates: Vec<Vec3> = map
        .spheres()
        .iter()
        .filter(|s| s.radius == radius)
        .map(|s| s.centre)
        .collect();
    let xs = [8, 9, 6, 3, 5].map(|i| Vec3::new(i as f32 * 2.0 - 12.0, 0.0, 0.0));
    assert_eq!(candidates, xs);
    assert!(map.spheres()[5..].iter().all(|s| s.radius == 2.0));
}

/// The corner masks: each cell keeps how many of its eight corner cells
/// are filled and which, bit 4x + 2y + z.
#[test]
fn the_corner_masks_count_the_filled_corner_cells() {
    let mut map = VoxelMap::new(0);
    assert!(map.resize(Vec3::ZERO, Vec3::ONE, [2, 2, 2]));
    map.set_cell([1, 1, 1], 1);
    map.set_cell([2, 1, 2], 1);
    mark_corners(&mut map);
    let fill = |c: [usize; 3]| map.fill()[map.index(c).unwrap()];
    assert_eq!(fill([0, 0, 0]), [1, 0b1000_0000]);
    assert_eq!(fill([1, 1, 1]), [2, 0b0010_0001]);
    assert_eq!(fill([1, 0, 1]), [2, 0b1000_0100]);
    assert_eq!(fill([2, 2, 2]), [0, 0]);
    // A grid with an axis of no cells gets no fill bytes.
    let mut flat = VoxelMap::new(0);
    assert!(flat.resize(Vec3::ZERO, Vec3::ONE, [2, 0, 2]));
    mark_corners(&mut flat);
    assert!(flat.fill().is_empty());
}

/// The neighbour tables: a neighbour's corners shift onto this cell's
/// (a corner bit is 4x + 2y + z) and only the shared ones are kept; the
/// mirror turns corner c into 7 − c.
#[test]
fn neighbours_share_their_coinciding_corners() {
    // The −x neighbour's x = 1 corners are this cell's x = 0 ones.
    assert_eq!(shared_corners(0b1111_0000, [-1, 0, 0]), 0b0000_1111);
    assert_eq!(shared_corners(0b0000_1111, [-1, 0, 0]), 0);
    assert_eq!(shared_corners(0b0000_0001, [1, 0, 0]), 0b0001_0000);
    assert_eq!(shared_corners(0b0000_0001, [0, 0, 1]), 0b0000_0010);
    assert_eq!(shared_corners(0b0000_0100, [0, -1, 0]), 0b0000_0001);
    // The far diagonal shares one corner: its 7 is this cell's 0.
    assert_eq!(shared_corners(0b1000_0000, [-1, -1, -1]), 0b0000_0001);
    assert_eq!(shared_corners(0xff, [0, 0, 0]), 0xff);
    assert_eq!(mirror_mask(0b0000_0001), 0b1000_0000);
    assert_eq!(mirror_mask(0b0000_0010), 0b0100_0000);
    assert_eq!(mirror_mask(0b0000_1111), 0b1111_0000);
    assert_eq!(mirror_mask(0b0011_0011), 0b1100_1100);
}

/// The fallback of a 6 × 6 × 2 slab, too thin for a candidate: each of the
/// 25 cells with eight corners gives a sphere one cell (2 cm) across. The
/// middle one sits on its corner point, the pulls of its empty neighbours
/// cancelling over all 26 of them; the one at x = 1 is pulled 24 half cells
/// (of 0.02 · 1 cm) toward +x by its sparser −x neighbours.
#[test]
fn a_thin_slab_takes_the_fallback_spheres() {
    let mut map = solid_map(Vec3::new(6.0, 6.0, 2.0), [6, 6, 2]);
    mark_corners(&mut map);
    extract_spheres(&mut map, &VolumeOptions::default());
    assert_eq!(map.spheres().len(), 25);
    assert!(map.spheres().iter().all(|s| s.radius == 2.0));
    let middle = map.spheres()[12];
    assert_eq!(middle.centre, Vec3::ZERO);
    let edge = map.spheres()[2];
    assert_eq!(edge.centre, Vec3::new(24.0 * 0.02 + (2.0 - 6.0), 0.0, 0.0));
}

/// The neighbour scans walk all 26 offsets but the centre, in x, y, z
/// order, the far corner `(+1, +1, +1)` last.
#[test]
fn the_neighbour_scans_read_all_26() {
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let expected = (x, y, z) != (0, 0, 0);
                assert_eq!(
                    NEIGHBOURS.iter().filter(|d| **d == [x, y, z]).count(),
                    usize::from(expected)
                );
            }
        }
    }
    assert_eq!(NEIGHBOURS[24], [1, 1, 0]);
    assert_eq!(NEIGHBOURS[25], [1, 1, 1]);
}

/// `mark_corners` walks the whole grid, its border rows included: every cell
/// from index 0 to the last border cell gets its count and mask.
#[test]
fn the_corner_masks_cover_the_border_rows() {
    let mut map = solid_map(Vec3::new(3.0, 3.0, 3.0), [2, 2, 2]);
    mark_corners(&mut map);
    let fill = |c: [usize; 3]| map.fill()[map.index(c).unwrap()];
    // Cell (1, 1, 1) is filled at all eight corners.
    assert_eq!(fill([1, 1, 1]), [8, 0xff]);
    // A cell at the high border on x has only its low x corner cells filled:
    // four of the eight, bits 0 to 3.
    assert_eq!(fill([2, 1, 1]), [4, 0b0000_1111]);
    assert_eq!(fill([3, 1, 1]), [0, 0]);
    // At z = 2 its four filled corners are the ones at z = 0 of the corner
    // offsets, bits 0, 2, 4 and 6.
    assert_eq!(fill([1, 1, 2]), [4, 0b0101_0101]);
}

/// The four point masses are 4 over the barycentric weights of the centre
/// (moved by the builder's origin) on the rest tetrahedron.
#[test]
fn the_volume_weighs_the_point_masses() {
    let rest = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
    let summary = VolumeSummary {
        centre: [0.25, 0.25, 0.25],
        mass: 1.0,
    };
    assert_eq!(summary.point_masses(Vec3::ZERO, rest), [16.0; 4]);
    // Weights ½, ¼, ⅛, ⅛: masses 8, 16, 32, 32.
    let summary = VolumeSummary {
        centre: [0.0, -0.125, 0.125],
        mass: 1.0,
    };
    assert_eq!(
        summary.point_masses(Vec3::new(0.25, 0.25, 0.0), rest),
        [8.0, 16.0, 32.0, 32.0]
    );
    // A flat tetrahedron gives no finite weights: equal masses.
    assert_eq!(
        summary.point_masses(
            Vec3::ZERO,
            [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 0.0)]
        ),
        [4.0; 4]
    );
    assert_eq!(
        barycentric_weights(Vec3::splat(0.25), rest),
        [0.25, 0.25, 0.25, 0.25]
    );
}
