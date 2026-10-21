use super::*;

/// A box mesh of half extents `half` around the origin, as 12 outward
/// triangles.
fn cube(half: Vec3) -> Vec<[Vec3; 3]> {
    let c = [
        Vec3::new(-half.x, -half.y, -half.z),
        Vec3::new(half.x, -half.y, -half.z),
        Vec3::new(half.x, half.y, -half.z),
        Vec3::new(-half.x, half.y, -half.z),
        Vec3::new(-half.x, -half.y, half.z),
        Vec3::new(half.x, -half.y, half.z),
        Vec3::new(half.x, half.y, half.z),
        Vec3::new(-half.x, half.y, half.z),
    ];
    let faces = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 7, 6],
        [3, 6, 2],
        [0, 4, 7],
        [0, 7, 3],
        [1, 2, 6],
        [1, 6, 5],
    ];
    faces.map(|f| f.map(|i| c[i])).to_vec()
}

fn mesh(half: Vec3) -> Mesh {
    Mesh::with_bounds(
        &cube(half),
        Bounds {
            centre: Vec3::ZERO,
            half_extents: half,
        },
    )
}

/// The axes come out smallest first, with the original's ties (x before z
/// when they are equal) and its NaN fall-through.
#[test]
fn extents_order_smallest_first() {
    assert_eq!(extent_order(Vec3::new(1.0, 2.0, 3.0)), [0, 1, 2]);
    assert_eq!(extent_order(Vec3::new(3.0, 2.0, 1.0)), [2, 1, 0]);
    assert_eq!(extent_order(Vec3::new(2.0, 1.0, 3.0)), [1, 0, 2]);
    assert_eq!(extent_order(Vec3::new(2.0, 3.0, 1.0)), [2, 0, 1]);
    assert_eq!(extent_order(Vec3::new(1.0, 3.0, 2.0)), [0, 2, 1]);
    assert_eq!(extent_order(Vec3::new(3.0, 1.0, 2.0)), [1, 2, 0]);
    // Ties keep x before z, y before z.
    assert_eq!(extent_order(Vec3::new(2.0, 1.0, 2.0)), [1, 0, 2]);
    assert_eq!(extent_order(Vec3::new(1.0, 2.0, 2.0)), [0, 1, 2]);
    assert_eq!(extent_order(Vec3::splat(f32::NAN)), [1, 0, 2]);
}

/// The smaller extents are lifted to a fifth of the largest (in double
/// precision), then every extent to six centimetres.
#[test]
fn extents_are_clamped() {
    // 10, 1, 20: y is lifted to 4, x stays.
    let order = extent_order(Vec3::new(10.0, 1.0, 20.0));
    assert_eq!(order, [1, 0, 2]);
    let clamped = clamp_extents(Vec3::new(10.0, 1.0, 20.0), order);
    assert_eq!(clamped, Vec3::new(10.0, 6.0, 20.0));
    // 10, 1, 10: y and z both under 2, both lifted.
    let order = extent_order(Vec3::new(10.0, 1.0, 10.0));
    assert_eq!(order, [1, 0, 2]);
    let clamped = clamp_extents(Vec3::new(10.0, 1.0, 10.0), order);
    assert_eq!(clamped, Vec3::new(10.0, 6.0, 10.0));
    // Everything under six centimetres.
    let order = extent_order(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(
        clamp_extents(Vec3::new(1.0, 2.0, 3.0), order),
        Vec3::splat(6.0)
    );
    // A NaN extent is left alone by both comparisons.
    let order = extent_order(Vec3::new(f32::NAN, 2.0, 3.0));
    let clamped = clamp_extents(Vec3::new(f32::NAN, 2.0, 3.0), order);
    assert!(clamped.x.is_nan() && clamped.y == 6.0 && clamped.z == 6.0);
}

/// A limb cuts its box at its origin and lays the tetrahedron over the cut
/// box; a mirrored limb on the body's +x side flips all four points, each
/// scaled and placed.
#[test]
fn a_limb_cuts_its_box_at_the_origin() {
    let node = NodeShape::new(
        Vec3::new(20.0, 0.0, 0.0),
        Vec3::splat(10.0),
        Vec3::new(20.0, 0.0, 0.0),
    );
    assert_eq!(limb_box(&node), Vec3::new(15.0, 10.0, 10.0));
    let shape = build_limb(
        &mesh(Vec3::splat(10.0)),
        &node,
        BuilderFlags {
            mirror: true,
            ..BuilderFlags::default()
        },
    );
    assert_eq!(shape.half_extents, Vec3::new(15.0, 10.0, 10.0));
    assert_eq!(shape.axis_order, [1, 2, 0]);
    let s = std::f32::consts::SQRT_2;
    let c = node.box_centre;
    let scaled = |u: Vec3| u * shape.half_extents + c;
    assert_eq!(shape.rest[0], scaled(Vec3::new(1.0, -s, 0.0)));
    assert_eq!(shape.rest[1], scaled(Vec3::new(1.0, s, 0.0)));
    assert_eq!(shape.rest[2], scaled(Vec3::new(-1.0, 0.0, s)));
    assert_eq!(shape.rest[3], scaled(Vec3::new(-1.0, 0.0, -s)));
    // A limb whose origin is at the model's centre takes the centre
    // limbs' order (x smallest, z middle, y largest).
    let centre = NodeShape::new(Vec3::ZERO, Vec3::splat(10.0), Vec3::new(1.0, 0.0, 0.0));
    let shape = build_limb(
        &mesh(Vec3::splat(10.0)),
        &centre,
        BuilderFlags {
            mirror: true,
            ..BuilderFlags::default()
        },
    );
    assert_eq!(shape.axis_order, [0, 2, 1]);
}

/// A root keeps its node box; the axes ascend (x, y, z here) and the sign
/// of the middle √2 makes every axis order the same handedness.
#[test]
fn a_root_lays_the_tetrahedron_over_its_box() {
    let node = NodeShape::new(Vec3::ZERO, Vec3::new(10.0, 20.0, 40.0), Vec3::ZERO);
    let shape = build_root(
        &mesh(Vec3::new(10.0, 20.0, 40.0)),
        &node,
        BuilderFlags::default(),
    );
    assert_eq!(shape.axis_order, [0, 1, 2]);
    let s = std::f32::consts::SQRT_2;
    let h = shape.half_extents;
    assert_eq!(shape.rest[0], Vec3::new(s * h.x, 0.0, -h.z));
    assert_eq!(shape.rest[1], Vec3::new(-s * h.x, 0.0, -h.z));
    assert_eq!(shape.rest[2], Vec3::new(0.0, s * h.y, h.z));
    assert_eq!(shape.rest[3], Vec3::new(0.0, -s * h.y, h.z));
    // The odd order (y, x, z) flips the middle sign.
    let node = NodeShape::new(Vec3::ZERO, Vec3::new(20.0, 10.0, 40.0), Vec3::ZERO);
    let shape = build_root(
        &mesh(Vec3::new(20.0, 10.0, 40.0)),
        &node,
        BuilderFlags::default(),
    );
    assert_eq!(shape.axis_order, [1, 0, 2]);
    let h = shape.half_extents;
    assert_eq!(shape.rest[2], Vec3::new(-s * h.x, 0.0, h.z));
    assert_eq!(shape.rest[3], Vec3::new(s * h.x, 0.0, h.z));
}

/// A mirrored root set off to the −x side grows its box over its
/// children's and takes the side's axis order (the side test reads the
/// distance, so −x counts), but its tetrahedron does not flip (that test
/// reads the sign).
#[test]
fn a_mirrored_root_on_the_left_takes_the_side_order() {
    let mut node = NodeShape::new(
        Vec3::new(-20.0, 0.0, 0.0),
        Vec3::new(8.0, 12.0, 10.0),
        Vec3::new(-20.0, 0.0, 0.0),
    );
    node.children = vec![Vec3::new(9.0, 1.0, 16.0)];
    let flags = BuilderFlags {
        mirror: true,
        ..BuilderFlags::default()
    };
    let shape = build_root(&mesh(Vec3::new(8.0, 12.0, 10.0)), &node, flags);
    assert_eq!(shape.half_extents, Vec3::new(9.0, 12.0, 16.0));
    assert_eq!(shape.axis_order, [1, 2, 0]);
    let s = std::f32::consts::SQRT_2;
    let h = shape.half_extents;
    let c = node.box_centre;
    // y smallest, z middle, x largest: p0 = +√2 y, −1 x; not flipped.
    assert_eq!(shape.rest[0], Vec3::new(-h.x, s * h.y, 0.0) + c);
    assert_eq!(shape.rest[3], Vec3::new(h.x, 0.0, -s * h.z) + c);
    // Unmirrored, the same node keeps its own box and order.
    let shape = build_root(
        &mesh(Vec3::new(8.0, 12.0, 10.0)),
        &node,
        BuilderFlags::default(),
    );
    assert_eq!(shape.half_extents, Vec3::new(8.0, 12.0, 10.0));
    assert_eq!(shape.axis_order, [0, 2, 1]);
}

/// The quality flag asks for a cell per centimetre and eight cells at
/// least; without it the mesh's smallest half extent chooses 0.3 or 0.6.
/// A weapon's record scales the resolution and fixes its own depth, and
/// the wide-query flag reaches every builder.
#[test]
fn the_fit_flags_reach_the_grid() {
    let small = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::new(3.0, 5.0, 20.0),
    };
    let large = Bounds {
        centre: Vec3::ZERO,
        half_extents: Vec3::new(20.0, 30.0, 40.0),
    };
    let plain = builder_options(&small, BuilderFlags::default());
    assert_eq!((plain.resolution, plain.min_depth), (0.6, 6));
    let fine = builder_options(&large, BuilderFlags::default());
    assert_eq!((fine.resolution, fine.min_depth), (0.3, 6));
    let precise = builder_options(
        &small,
        BuilderFlags {
            precise: true,
            ..BuilderFlags::default()
        },
    );
    assert_eq!((precise.resolution, precise.min_depth), (1.0, 8));
    let wide = builder_options(
        &small,
        BuilderFlags {
            wide_queries: true,
            ..BuilderFlags::default()
        },
    );
    assert!(wide.wide_queries);
    let weapon = weapon_options(&small, &WeaponFit::SHIPPED, BuilderFlags::default());
    assert_eq!(weapon.resolution, 0.6 * WEAPON_RESOLUTION_SCALE);
    assert_eq!((weapon.min_depth, weapon.even_axes), (6, true));
    assert_eq!(weapon.min_free_corners, 0);
    let weapon = weapon_options(
        &small,
        &WeaponFit::SHIPPED,
        BuilderFlags {
            precise: true,
            wide_queries: true,
            ..BuilderFlags::default()
        },
    );
    assert_eq!(weapon.resolution, 0.6 * WEAPON_RESOLUTION_SCALE);
    assert!(weapon.wide_queries);
}

/// The sphere sort takes the nearest sphere first, a tie going to the one
/// farther from the previous pick; a sphere over 1e8 squared from the
/// centre is never chosen, and the array keeps its length by repeating the
/// next entry (the original's fallback).
#[test]
fn spheres_are_sorted_around_their_centre() {
    let sphere = |x: f32, y: f32| Sphere {
        centre: Vec3::new(x, y, 0.0),
        radius: 1.0,
    };
    let mut list = vec![sphere(3.0, 0.0), sphere(4.0, 0.0), sphere(0.0, 0.0)];
    sort_spheres(&mut list, Vec3::ZERO);
    assert_eq!(
        list.iter().map(|s| s.centre.x).collect::<Vec<_>>(),
        [0.0, 3.0, 4.0]
    );
    // Equal distances: the later index wins the first pick, then the other.
    let mut list = vec![sphere(1.0, 0.0), sphere(0.0, 1.0)];
    sort_spheres(&mut list, Vec3::ZERO);
    assert_eq!(list[0].centre, Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(list[1].centre, Vec3::new(1.0, 0.0, 0.0));
    // Too far: the far sphere is never chosen and the next entry repeats.
    let mut list = vec![sphere(1e5, 0.0), sphere(0.0, 0.0), sphere(2.0, 0.0)];
    sort_spheres(&mut list, Vec3::ZERO);
    assert_eq!(
        list.iter().map(|s| s.centre.x).collect::<Vec<_>>(),
        [0.0, 2.0, 2.0]
    );
}

/// Gravity shares are 4/mᵢ scaled by 4 over their sum over all four
/// masses: equal masses share equally, masses 2, 4, 8, 8 give 2, 1, ½, ½.
#[test]
fn gravity_shares_sum_to_four() {
    assert_eq!(setup_gravity([4.0; 4]), [1.0; 4]);
    assert_eq!(setup_gravity([2.0, 4.0, 8.0, 8.0]), [2.0, 1.0, 0.5, 0.5]);
    // 4/3, 4/5, 4/7, 4/9 sum to 2.9841…; the quotient 4/sum is taken first.
    let inverse = [4.0f32 / 3.0, 4.0 / 5.0, 4.0 / 7.0, 4.0 / 9.0];
    let sum = ((inverse[0] + inverse[1]) + inverse[2]) + inverse[3];
    let scale = 4.0 / sum;
    assert_eq!(
        setup_gravity([3.0, 5.0, 7.0, 9.0]),
        inverse.map(|w| w * scale)
    );
    // No usable masses: one each (Exarust's guard).
    assert_eq!(setup_gravity([0.0, 4.0, 4.0, 4.0]), [1.0; 4]);
}

/// A built shape weighs the body from its volume and its rest points span
/// the box; the spheres are the fit's.
#[test]
fn a_built_shape_weighs_its_body() {
    let node = NodeShape::new(Vec3::ZERO, Vec3::new(10.0, 20.0, 30.0), Vec3::ZERO);
    let shape = build_root(
        &mesh(Vec3::new(10.0, 20.0, 30.0)),
        &node,
        BuilderFlags::default(),
    );
    assert!(shape.volume > 0.0, "{shape:?}");
    assert_eq!(shape.mobility, volume::MOBILITY_VOLUME / shape.volume);
    assert!(!shape.spheres.is_empty());
    assert!(shape.rest.iter().all(|p| p.is_finite()));
    assert!(shape.point_masses.iter().all(|m| m.is_finite()));
    assert!(shape.counts.iter().all(|&c| (1..=96).contains(&c)));
    // A 20 × 40 × 60 cm box holds 48 000 cm³; the voxel fit is within a
    // few percent of it.
    assert!(
        (shape.volume - 48_000.0).abs() < 4_000.0,
        "{}",
        shape.volume
    );
}
