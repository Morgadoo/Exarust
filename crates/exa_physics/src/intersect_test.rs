//! Tests for the intersection library, pinned to numbers worked by hand
//! from the original's instructions. Dimensions are the original's
//! centimetres.

use super::*;

fn close(a: f32, b: f32) {
    assert!(
        (a - b).abs() <= 1e-5 * (1.0 + b.abs()),
        "{a} is not close to {b}"
    );
}

fn close_vec(a: Vec3, b: Vec3) {
    close(a.x, b.x);
    close(a.y, b.y);
    close(a.z, b.z);
}

fn unit_box() -> ConvexVolume {
    ConvexVolume {
        planes: vec![
            Plane::new(Vec3::X, -1.0),
            Plane::new(-Vec3::X, -1.0),
            Plane::new(Vec3::Y, -1.0),
            Plane::new(-Vec3::Y, -1.0),
            Plane::new(-Vec3::Z, -1.0),
            Plane::new(Vec3::Z, -1.0),
        ],
    }
}

/// A cube of half side `h` about the origin with a polygon on every face,
/// wound counter-clockwise seen from outside and refreshed, in the face
/// order `normals`. Face n has the corners n + s·u + t·v (×h) with
/// u × v = n, in the order (−1, 1), (−1, −1), (1, −1), (1, 1).
fn cube(h: f32, normals: [Vec3; 6]) -> ConvexVolume {
    let frame = |n: Vec3| {
        if n == Vec3::X {
            (Vec3::Y, Vec3::Z)
        } else if n == Vec3::NEG_X {
            (Vec3::Z, Vec3::Y)
        } else if n == Vec3::Y {
            (Vec3::Z, Vec3::X)
        } else if n == Vec3::NEG_Y {
            (Vec3::X, Vec3::Z)
        } else if n == Vec3::Z {
            (Vec3::X, Vec3::Y)
        } else {
            (Vec3::Y, Vec3::X)
        }
    };
    let mut volume = ConvexVolume {
        planes: normals
            .iter()
            .map(|&n| {
                let (u, v) = frame(n);
                let corner = |s: f32, t: f32| (n + u * s + v * t) * h;
                Plane {
                    normal: Vec3::ZERO,
                    offset: 0.0,
                    vertices: vec![
                        corner(-1.0, 1.0),
                        corner(-1.0, -1.0),
                        corner(1.0, -1.0),
                        corner(1.0, 1.0),
                    ],
                }
            })
            .collect(),
    };
    volume.refresh();
    volume
}

const FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

#[test]
fn the_processor_minimum_and_maximum_pick_the_second_on_nan() {
    assert_eq!(min_first(1.0, 2.0), 1.0);
    assert_eq!(min_first(2.0, 1.0), 1.0);
    assert_eq!(max_first(1.0, 2.0), 2.0);
    assert!(min_first(1.0, f32::NAN).is_nan());
    assert_eq!(min_first(f32::NAN, 1.0), 1.0);
    assert!(max_first(1.0, f32::NAN).is_nan());
    assert_eq!(max_first(f32::NAN, 1.0), 1.0);
}

#[test]
fn a_sweep_takes_its_length_and_direction() {
    let sweep = Sweep::new(Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0));
    close(sweep.length, 5.0);
    close_vec(sweep.direction, Vec3::new(0.6, 0.8, 0.0));
    close(sweep.inverse_direction.x, 5.0 / 3.0);
    close(sweep.inverse_direction.y, 1.25);
    assert!(sweep.inverse_direction.z.is_infinite());
}

#[test]
fn a_zero_length_sweep_is_nan() {
    // The original's reciprocal square root of zero is infinite, so the
    // length (squared × inverse) and the direction are 0 × ∞, NaN.
    let sweep = Sweep::new(Vec3::ONE, Vec3::ONE);
    assert!(sweep.length.is_nan());
    assert!(sweep.direction.is_nan());
}

#[test]
fn point_to_segment_clamps_before_measuring() {
    let origin = Vec3::ZERO;
    let along_x = Vec3::X;
    // Beside the segment: the perpendicular is the whole offset.
    close(
        point_segment_distance_squared(Vec3::new(0.0, 5.0, 0.0), origin, along_x, 10.0),
        25.0,
    );
    // Past the end: the clamped end is (10, 0, 0), offset (5, 3, 4).
    close(
        point_segment_distance_squared(Vec3::new(15.0, 3.0, 4.0), origin, along_x, 10.0),
        50.0,
    );
    // Before the start: the start is the closest point, offset (-3, 3, 4).
    close(
        point_segment_distance_squared(Vec3::new(-3.0, 3.0, 4.0), origin, along_x, 10.0),
        34.0,
    );
    // On the segment, and exactly on the far end.
    close(
        point_segment_distance_squared(Vec3::new(4.0, 0.0, 0.0), origin, along_x, 10.0),
        0.0,
    );
    close(
        point_segment_distance_squared(Vec3::new(10.0, 0.0, 0.0), origin, along_x, 10.0),
        0.0,
    );
}

#[test]
fn sphere_contacts_count_as_overlapping() {
    // Centres 10 apart, reach 7: apart.
    assert!(spheres_separated(
        Vec3::ZERO,
        3.0,
        Vec3::new(10.0, 0.0, 0.0),
        4.0
    ));
    // Centres 7 apart, reach 7: touching, not apart.
    assert!(!spheres_separated(
        Vec3::ZERO,
        3.0,
        Vec3::new(7.0, 0.0, 0.0),
        4.0
    ));
    // One sphere inside the other.
    assert!(!spheres_separated(Vec3::ZERO, 4.0, Vec3::ONE, 1.0));
    // 2.5 apart, reach 2: apart.
    assert!(spheres_separated(
        Vec3::ZERO,
        1.0,
        Vec3::new(2.5, 0.0, 0.0),
        1.0
    ));
}

#[test]
fn point_to_aabb_is_zero_inside_and_sums_the_overhangs() {
    let (centre, extents) = (Vec3::ZERO, Vec3::new(2.0, 3.0, 4.0));
    // One axis out by 3.
    close(
        point_aabb_distance_squared(Vec3::new(5.0, 0.0, 0.0), centre, extents),
        9.0,
    );
    // On the box, and inside it.
    close(
        point_aabb_distance_squared(Vec3::new(2.0, 3.0, 4.0), centre, extents),
        0.0,
    );
    close(
        point_aabb_distance_squared(Vec3::new(1.0, 2.0, 3.0), centre, extents),
        0.0,
    );
    // Overhangs 2, 1, 1.
    close(
        point_aabb_distance_squared(Vec3::new(4.0, 4.0, 5.0), centre, extents),
        6.0,
    );
}

#[test]
fn sphere_and_aabb_are_apart_only_beyond_the_reach() {
    let (centre, extents) = (Vec3::ZERO, Vec3::splat(2.0));
    // 8 beyond the face, radius 1.
    assert!(sphere_aabb_separated(
        Vec3::new(10.0, 0.0, 0.0),
        1.0,
        centre,
        extents
    ));
    // Exactly touching the face.
    assert!(!sphere_aabb_separated(
        Vec3::new(3.0, 0.0, 0.0),
        1.0,
        centre,
        extents
    ));
    // Half a centimetre into the face.
    assert!(!sphere_aabb_separated(
        Vec3::new(2.5, 0.0, 0.0),
        1.0,
        centre,
        extents
    ));
    // Around the corner, overhangs (0.5, 0.5): distance² 0.5.
    assert!(!sphere_aabb_separated(
        Vec3::new(2.5, 2.5, 0.0),
        1.0,
        centre,
        extents
    ));
    // Around the corner, overhangs (1.5, 1.5): distance² 4.5.
    assert!(sphere_aabb_separated(
        Vec3::new(3.5, 3.5, 0.0),
        1.0,
        centre,
        extents
    ));
    // A point sphere on the face touches (0 − 0 is +0, no sign bit); one
    // beyond it is apart.
    assert!(!sphere_aabb_separated(
        Vec3::new(2.0, 0.0, 0.0),
        0.0,
        centre,
        extents
    ));
    assert!(sphere_aabb_separated(
        Vec3::new(3.0, 0.0, 0.0),
        0.0,
        centre,
        extents
    ));
}

#[test]
fn a_capsule_and_a_sphere_touch_at_their_combined_reach() {
    let sweep = Sweep::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
    // Perpendicular 3, reach 1 + 1.
    assert!(sphere_segment_separated(
        &sweep,
        1.0,
        Vec3::new(5.0, 3.0, 0.0),
        1.0
    ));
    // Perpendicular 2, reach 2: touching.
    assert!(!sphere_segment_separated(
        &sweep,
        1.0,
        Vec3::new(5.0, 2.0, 0.0),
        1.0
    ));
    // Past the end, 2 beyond the end point, reach 2: touching.
    assert!(!sphere_segment_separated(
        &sweep,
        1.0,
        Vec3::new(12.0, 0.0, 0.0),
        1.0
    ));
    // Just off that: distance² 4.01.
    assert!(sphere_segment_separated(
        &sweep,
        1.0,
        Vec3::new(12.0, 0.1, 0.0),
        1.0
    ));
    // Before the start, 5 away, reach 2.
    assert!(sphere_segment_separated(
        &sweep,
        1.0,
        Vec3::new(-5.0, 0.0, 0.0),
        1.0
    ));
    // Over the start, 1.5 away, reach 2.
    assert!(!sphere_segment_separated(
        &sweep,
        1.0,
        Vec3::new(-1.5, 0.0, 0.0),
        1.0
    ));
}

#[test]
fn a_swept_sphere_hits_an_aabb_when_the_slabs_overlap() {
    let box_centre = Vec3::ZERO;
    let extents = Vec3::splat(2.0);
    // Through the middle: along x the slab is 7..13 of a length of 20.
    let through = Sweep::new(Vec3::new(-10.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0));
    assert!(swept_sphere_aabb_hit(&through, 1.0, box_centre, extents));
    // Six up, beyond the inflated extent of 3.
    let above = Sweep::new(Vec3::new(-10.0, 6.0, 0.0), Vec3::new(10.0, 6.0, 0.0));
    assert!(!swept_sphere_aabb_hit(&above, 1.0, box_centre, extents));
    // Parallel to the y axis, through the middle.
    let along_y = Sweep::new(Vec3::new(0.0, -10.0, 0.0), Vec3::new(0.0, 10.0, 0.0));
    assert!(swept_sphere_aabb_hit(&along_y, 1.0, box_centre, extents));
    // Starting inside: the entry clamps to zero, the exit stays positive.
    let inside = Sweep::new(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    assert!(swept_sphere_aabb_hit(&inside, 1.0, box_centre, extents));
    // An end short of the inflated −x face: the entry 7 is past the
    // length 6.
    let ending_short = Sweep::new(Vec3::new(-10.0, 0.0, 0.0), Vec3::new(-4.0, 0.0, 0.0));
    assert!(!swept_sphere_aabb_hit(
        &ending_short,
        1.0,
        box_centre,
        extents
    ));
    // Grazing the inflated face at y = 3: the y slab's far side is
    // 0 × ∞ = NaN. The processor's maximum keeps that NaN in the exit
    // lanes, its minimum then drops it for the z lane's +∞, so the exit is
    // +∞ and the sweep hits.
    let grazing = Sweep::new(Vec3::new(-10.0, 3.0, 0.0), Vec3::new(10.0, 3.0, 0.0));
    assert!(swept_sphere_aabb_hit(&grazing, 1.0, box_centre, extents));
    // One centimetre further out: both y sides are −∞, so the exit is −∞.
    let missing = Sweep::new(Vec3::new(-10.0, 4.0, 0.0), Vec3::new(10.0, 4.0, 0.0));
    assert!(!swept_sphere_aabb_hit(&missing, 1.0, box_centre, extents));
}

#[test]
fn a_swept_sphere_hits_an_obb_in_its_own_frame() {
    let obb = Obb::axis_aligned(Vec3::ZERO, Vec3::splat(2.0));
    let through = (Vec3::new(-10.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0));
    assert!(swept_sphere_obb_hit(through.0, through.1, 1.0, &obb));
    let above = (Vec3::new(-10.0, 4.0, 0.0), Vec3::new(10.0, 4.0, 0.0));
    assert!(!swept_sphere_obb_hit(above.0, above.1, 1.0, &obb));

    // The same square turned 45°: its world y extent is 2√2 ≈ 2.828. At
    // y = 2.8 the slabs are 0.2186..0.5014 and 0.4986..0.7814 of the
    // travel; at y = 3, 0.2086..0.4914 and 0.5086..0.7914.
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let turned = Obb::new(
        Vec3::ZERO,
        Vec3::splat(2.0),
        [Vec3::new(s, s, 0.0), Vec3::new(-s, s, 0.0), Vec3::Z],
    );
    let at = |y: f32| (Vec3::new(-10.0, y, 0.0), Vec3::new(10.0, y, 0.0));
    let (a, b) = at(0.0);
    assert!(swept_sphere_obb_hit(a, b, 0.0, &turned));
    let (a, b) = at(2.8);
    assert!(swept_sphere_obb_hit(a, b, 0.0, &turned));
    let (a, b) = at(3.0);
    assert!(!swept_sphere_obb_hit(a, b, 0.0, &turned));
}

#[test]
fn a_segment_hits_an_obb_by_the_slab_test() {
    let obb = Obb::axis_aligned(Vec3::ZERO, Vec3::splat(2.0));
    let along = Vec3::new(20.0, 0.0, 0.0);
    assert!(segment_obb_hit(Vec3::new(-10.0, 0.0, 0.0), along, &obb));
    assert!(!segment_obb_hit(Vec3::new(-10.0, 3.0, 0.0), along, &obb));
    // The entry exactly at the end of the segment (8 of 8) is rejected.
    assert!(!segment_obb_hit(
        Vec3::new(-10.0, 0.0, 0.0),
        Vec3::new(8.0, 0.0, 0.0),
        &obb
    ));
    // Along the y = 2 face: the y slab's far side is 0 × ∞ = NaN, which
    // the processor's minimum carries into the entry fold and its maximum
    // into the exit fold. The fold then lands on the z lane's −∞ entry and
    // +∞ exit, so the segment hits.
    assert!(segment_obb_hit(Vec3::new(-10.0, 2.0, 0.0), along, &obb));
    // Along the z = 2 face the NaN is the z lane's high side: it is the
    // entry fold's last lane, so the entry is NaN, and the exit fold ends
    // on it too. The clamped entry 0 is not before a NaN exit (an ordered
    // compare): a miss.
    assert!(!segment_obb_hit(Vec3::new(-10.0, 0.0, 2.0), along, &obb));
}

#[test]
fn point_to_obb_distance_turns_with_the_box() {
    let axis_aligned = Obb::axis_aligned(Vec3::ZERO, Vec3::new(2.0, 3.0, 4.0));
    close(
        point_obb_distance_squared(Vec3::new(5.0, 0.0, 0.0), &axis_aligned),
        9.0,
    );
    close(
        point_obb_distance_squared(Vec3::new(1.0, 2.0, 3.0), &axis_aligned),
        0.0,
    );
    // Turned 45° about z: the local coordinates of (5, 0, 0) are
    // (5/√2, −5/√2, 0), so the overhang is 5/√2 − 2 on each of two axes.
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let turned = Obb::new(
        Vec3::ZERO,
        Vec3::splat(2.0),
        [Vec3::new(s, s, 0.0), Vec3::new(-s, s, 0.0), Vec3::Z],
    );
    let over = 5.0 * s - 2.0;
    close(
        point_obb_distance_squared(Vec3::new(5.0, 0.0, 0.0), &turned),
        2.0 * over * over,
    );
    // A NaN overhang's bits are past every extent's, so it is counted.
    assert!(point_obb_distance_squared(Vec3::new(f32::NAN, 0.0, 0.0), &axis_aligned).is_nan());
}

#[test]
fn a_sphere_meets_a_segment_at_its_ends() {
    let start = Vec3::ZERO;
    let vector = Vec3::new(10.0, 0.0, 0.0);
    // Midway, 3 above: distance² 9 > 4.
    assert!(!segment_sphere(
        start,
        vector,
        Vec3::new(5.0, 3.0, 0.0),
        2.0
    ));
    // Midway, 2 above: touching.
    assert!(segment_sphere(start, vector, Vec3::new(5.0, 2.0, 0.0), 2.0));
    // Beyond the end by 2: touching the end point.
    assert!(segment_sphere(
        start,
        vector,
        Vec3::new(12.0, 0.0, 0.0),
        2.0
    ));
    // Beyond the end by 3.
    assert!(!segment_sphere(
        start,
        vector,
        Vec3::new(13.0, 0.0, 0.0),
        2.0
    ));
    // Behind the start: distance 5.
    assert!(!segment_sphere(
        start,
        vector,
        Vec3::new(-3.0, 4.0, 0.0),
        5.0 - 0.1
    ));
    assert!(segment_sphere(
        start,
        vector,
        Vec3::new(-3.0, 4.0, 0.0),
        5.0
    ));
    // A zero vector is the start point.
    assert!(segment_sphere(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(1.0, 1.0, 0.0),
        2.0
    ));
    // A NaN projection measures from the start: distance² 1 ≤ 4.
    assert!(segment_sphere(
        Vec3::ZERO,
        Vec3::new(f32::NAN, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        2.0
    ));
}

#[test]
fn the_separating_axis_test_rejects_a_segment_above_the_box() {
    let obb = Obb::axis_aligned(Vec3::ZERO, Vec3::splat(2.0));
    // Through the middle.
    assert!(segment_obb_overlap(
        Vec3::new(-10.0, 0.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        &obb
    ));
    // Above it, parallel: the start is outside the y slab and not moving
    // toward it (10 × 0 ≥ 0).
    assert!(!segment_obb_overlap(
        Vec3::new(-10.0, 3.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        &obb
    ));
    // Inside the y slab.
    assert!(segment_obb_overlap(
        Vec3::new(-10.0, 1.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        &obb
    ));
    // Starting beyond the +x face and moving further away.
    assert!(!segment_obb_overlap(
        Vec3::new(3.0, 0.0, 0.0),
        Vec3::X * 10.0,
        &obb
    ));
    // Diagonal through the centre.
    assert!(segment_obb_overlap(
        Vec3::new(-10.0, -10.0, 0.0),
        Vec3::new(20.0, 20.0, 0.0),
        &obb
    ));
    // Starting inside.
    assert!(segment_obb_overlap(Vec3::ZERO, Vec3::X * 10.0, &obb));
    // A NaN travel separates on no axis: 10 above the box, yet not apart.
    assert!(segment_obb_overlap(
        Vec3::new(0.0, 10.0, 0.0),
        Vec3::new(f32::NAN, 0.0, 0.0),
        &obb
    ));
}

#[test]
fn bounds_cover_every_point_with_a_floor() {
    let points = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(5.0, -2.0, 7.0)];
    let (centre, extents) = bounds_of_points(&points);
    assert_eq!(centre, Vec3::new(3.0, 0.0, 5.0));
    assert_eq!(extents, Vec3::new(2.0, 2.0, 2.0));
    // A flat set has the floor on its flat axis.
    let flat = [Vec3::new(0.0, 1.0, 0.0), Vec3::new(4.0, 1.0, 2.0)];
    let (centre, extents) = bounds_of_points(&flat);
    assert_eq!(centre, Vec3::new(2.0, 1.0, 1.0));
    assert_eq!(extents, Vec3::new(2.0, MIN_BOUNDS_EXTENT, 1.0));
    // One point and no points are Exarust's guards.
    let (centre, extents) = bounds_of_points(&[Vec3::splat(2.0)]);
    assert_eq!(centre, Vec3::splat(2.0));
    assert_eq!(extents, Vec3::splat(MIN_BOUNDS_EXTENT));
    let (centre, extents) = bounds_of_points(&[]);
    assert_eq!(centre, Vec3::ZERO);
    assert_eq!(extents, Vec3::splat(MIN_BOUNDS_EXTENT));
}

#[test]
fn bounds_grow_to_include_a_point() {
    let (mut low, mut high) = (Vec3::ONE, Vec3::splat(2.0));
    expand_bounds(&mut low, &mut high, Vec3::new(0.0, 3.0, 1.5));
    assert_eq!(low, Vec3::new(0.0, 1.0, 1.0));
    assert_eq!(high, Vec3::new(2.0, 3.0, 2.0));
    // A point inside changes nothing.
    expand_bounds(&mut low, &mut high, Vec3::new(1.0, 1.0, 1.0));
    assert_eq!(low, Vec3::new(0.0, 1.0, 1.0));
    assert_eq!(high, Vec3::new(2.0, 3.0, 2.0));
}

#[test]
fn a_volume_relative_to_a_point_shifts_its_offsets() {
    let mut volume = unit_box();
    volume.planes[0].vertices = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
    let shifted = volume.relative_to(Vec3::new(2.0, 0.0, 0.0));
    // The +X face, x = 1, is at x = −1 seen from (2, 0, 0): offset −1 + 2.
    close(shifted.planes[0].offset, 1.0);
    assert!(shifted.planes[0].vertices.is_empty());
    // The −X face, x = −1, is at x = −3: offset −1 − 2.
    close(shifted.planes[1].offset, -3.0);
    // The z faces are orthogonal to the shift and keep their offsets.
    close(shifted.planes[4].offset, -1.0);
    // The origin seen from (2, 0, 0) is (−2, 0, 0), inside every face.
    assert!(
        shifted
            .planes
            .iter()
            .all(|p| p.distance(Vec3::new(-2.0, 0.0, 0.0)) <= 0.0)
    );
}

#[test]
fn refreshing_a_volume_rebuilds_its_planes_from_its_polygons() {
    let face = |vertices: Vec<Vec3>| ConvexVolume {
        planes: vec![Plane {
            normal: Vec3::ZERO,
            offset: 0.0,
            vertices,
        }],
    };
    // Counter-clockwise seen from +z: (v2 − v1) × (v0 − v1) =
    // (0, 1, 0) × (−1, 0, 0) = (0, 0, 1), offset −(n · v0) = −1.
    let mut ccw = face(vec![
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ]);
    ccw.refresh();
    close_vec(ccw.planes[0].normal, Vec3::Z);
    close(ccw.planes[0].offset, -1.0);
    // Clockwise seen from +z: (−1, 1, 0) × (0, 1, 0) = (0, 0, −1), offset 1.
    let mut cw = face(vec![
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
    ]);
    cw.refresh();
    close_vec(cw.planes[0].normal, -Vec3::Z);
    close(cw.planes[0].offset, 1.0);
    // A degenerate polygon gets a zero normal, as the original's
    // normalisation leaves a zero vector zero.
    let mut flat = face(vec![Vec3::ONE, Vec3::ONE, Vec3::ONE]);
    flat.refresh();
    assert_eq!(flat.planes[0].normal, Vec3::ZERO);
    // A face with too few vertices is left alone.
    let mut poor = ConvexVolume {
        planes: vec![Plane {
            normal: Vec3::X,
            offset: 2.0,
            vertices: vec![Vec3::ZERO, Vec3::X],
        }],
    };
    poor.refresh();
    assert_eq!(poor.planes[0].normal, Vec3::X);
    assert_eq!(poor.planes[0].offset, 2.0);
}

#[test]
fn the_test_cube_faces_outward() {
    let volume = cube(10.0, FACES);
    for (plane, normal) in volume.planes.iter().zip(FACES) {
        close_vec(plane.normal, normal);
        close(plane.offset, -10.0);
    }
}

#[test]
fn cloning_a_volume_copies_its_polygons() {
    let mut volume = unit_box();
    volume.planes[0].vertices = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
    let copy = volume.clone();
    volume.planes[0].vertices[0] = Vec3::splat(7.0);
    assert_eq!(copy.planes[0].vertices[0], Vec3::ZERO);
    assert_eq!(copy.planes.len(), volume.planes.len());
}

#[test]
fn a_volume_meets_an_obb_conservatively() {
    let volume = unit_box();
    let far = |x: f32| Obb::axis_aligned(Vec3::new(x, 0.0, 0.0), Vec3::splat(0.5));
    // A box's bounding sphere reaches its corners: 0.5·√3.
    let radius = 0.5 * 3.0f32.sqrt();
    assert!(volume.intersects_obb(&far(0.0), radius));
    // The box crosses the +X face.
    assert!(volume.intersects_obb(&far(1.2), radius));
    // The support along +X is 0.5, so the face distance 0.6 rejects it.
    assert!(!volume.intersects_obb(&far(1.6), radius));
    // The sphere test rejects from 0.866 out.
    assert!(!volume.intersects_obb(&far(2.0), radius));
    assert!(!volume.intersects_obb(&far(5.0), radius));

    // A square turned 45° has a support of √2·0.5 ≈ 0.707 along x.
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let turned = Obb::new(
        Vec3::new(1.2, 0.0, 0.0),
        Vec3::splat(0.5),
        [Vec3::new(s, s, 0.0), Vec3::new(-s, s, 0.0), Vec3::Z],
    );
    assert!(volume.intersects_obb(&turned, radius));
    let far = Obb::new(
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::splat(0.5),
        [Vec3::new(s, s, 0.0), Vec3::new(-s, s, 0.0), Vec3::Z],
    );
    assert!(!volume.intersects_obb(&far, radius));
}

#[test]
fn a_volume_meets_an_aabb_conservatively() {
    let volume = unit_box();
    let radius = 0.5 * 3.0f32.sqrt();
    assert!(volume.intersects_aabb(Vec3::ZERO, radius, Vec3::splat(0.5)));
    assert!(volume.intersects_aabb(Vec3::new(1.2, 0.0, 0.0), radius, Vec3::splat(0.5)));
    assert!(!volume.intersects_aabb(Vec3::new(1.6, 0.0, 0.0), radius, Vec3::splat(0.5)));
    assert!(!volume.intersects_aabb(Vec3::new(5.0, 0.0, 0.0), radius, Vec3::splat(0.5)));
    // A large radius passes the sphere test but not the tighter support
    // test, which is the original's order.
    assert!(!volume.intersects_aabb(Vec3::new(5.0, 0.0, 0.0), 10.0, Vec3::splat(0.5)));
}

#[test]
fn a_volume_reports_how_far_a_box_lies_outside_a_face() {
    let volume = unit_box();
    let at = |x: f32| Obb::axis_aligned(Vec3::new(x, 0.0, 0.0), Vec3::splat(0.5));
    close(volume.outside_distance(&at(0.0)), 0.0);
    close(volume.outside_distance(&at(1.2)), 0.0);
    close(volume.outside_distance(&at(1.6)), 0.1);
    close(volume.outside_distance(&at(2.0)), 0.5);
    close(volume.outside_distance(&at(5.0)), 3.5);
    // On the other side, the −X face gives the same 0.5.
    close(volume.outside_distance(&at(-2.0)), 0.5);
}

#[test]
fn clipping_a_polygon_builds_the_pyramid_faces() {
    let volume = unit_box();
    // The square at z = 1 lies on the +Z face, so no plane cuts it. It is
    // counter-clockwise seen from +z, clockwise seen from the apex.
    let polygon = [
        Vec3::new(-0.5, -0.5, 1.0),
        Vec3::new(0.5, -0.5, 1.0),
        Vec3::new(0.5, 0.5, 1.0),
        Vec3::new(-0.5, 0.5, 1.0),
    ];
    let apex = Vec3::ZERO;
    let pyramid = volume.clip_polygon(&polygon, apex).unwrap();
    assert_eq!(pyramid.planes.len(), 4);
    // Face i is (p[i] − apex) × (p[i − 1] − apex), normalised: face 0 is
    // (−0.5, −0.5, 1) × (−0.5, 0.5, 1) = (−1, 0, −0.5) → (−2, 0, −1)/√5.
    // The normals point into the pyramid's outside: (0, 0, 0.5) is inside.
    let root = 1.0 / 5.0f32.sqrt();
    let normals = [
        Vec3::new(-2.0 * root, 0.0, -root),
        Vec3::new(0.0, -2.0 * root, -root),
        Vec3::new(2.0 * root, 0.0, -root),
        Vec3::new(0.0, 2.0 * root, -root),
    ];
    for (i, plane) in pyramid.planes.iter().enumerate() {
        close_vec(plane.normal, normals[i]);
        close(plane.offset, 0.0);
        assert_eq!(plane.vertices.len(), 3);
        assert_eq!(plane.vertices[0], polygon[(i + 3) % 4]);
        assert_eq!(plane.vertices[1], apex);
        assert_eq!(plane.vertices[2], polygon[i]);
        assert!(plane.distance(Vec3::new(0.0, 0.0, 0.5)) < 0.0);
    }

    // A polygon wholly beyond the +X face leaves nothing.
    let outside = [
        Vec3::new(4.0, -0.5, 0.0),
        Vec3::new(5.0, -0.5, 0.0),
        Vec3::new(5.0, 0.5, 0.0),
        Vec3::new(4.0, 0.5, 0.0),
    ];
    assert!(volume.clip_polygon(&outside, Vec3::ZERO).is_none());
}

#[test]
fn clipping_a_polygon_by_a_face_keeps_the_crossing_points() {
    let volume = unit_box();
    // A square from x = −2 to 2, cut down to the box at x = ±1 and y = ±1.
    let polygon = [
        Vec3::new(-2.0, -2.0, 0.0),
        Vec3::new(2.0, -2.0, 0.0),
        Vec3::new(2.0, 2.0, 0.0),
        Vec3::new(-2.0, 2.0, 0.0),
    ];
    let pyramid = volume
        .clip_polygon(&polygon, Vec3::new(0.0, 0.0, -5.0))
        .unwrap();
    assert_eq!(pyramid.planes.len(), 4);
    // Every face's polygon ends on the clipped square's edge.
    let mut ends: Vec<Vec3> = pyramid.planes.iter().map(|p| p.vertices[2]).collect();
    ends.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    assert_eq!(
        ends,
        vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ]
    );
    // Each face is a plane through the apex; its offset follows.
    for plane in &pyramid.planes {
        close(plane.offset, -plane.normal.dot(plane.vertices[0]));
    }
}

#[test]
fn clipping_keeps_only_the_first_inner_run() {
    // The original keeps the run from the first entry to the next exit: a
    // zigzag dipping under y = 0 twice keeps its first dip only. The last
    // vertex (4, 1) is outside; (1, −1) enters with the crossing
    // (0.5, 0, 0) and (2, 1) leaves with (1.5, 0, 0).
    let below = ConvexVolume {
        planes: vec![Plane::new(Vec3::Y, 0.0)],
    };
    let zigzag = [
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, -1.0, 0.0),
        Vec3::new(2.0, 1.0, 0.0),
        Vec3::new(3.0, -1.0, 0.0),
        Vec3::new(4.0, 1.0, 0.0),
    ];
    let pyramid = below
        .clip_polygon(&zigzag, Vec3::new(0.0, 0.0, 5.0))
        .unwrap();
    let ends: Vec<Vec3> = pyramid.planes.iter().map(|p| p.vertices[2]).collect();
    assert_eq!(
        ends,
        vec![
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.5, 0.0, 0.0),
        ]
    );
}

#[test]
fn cutting_a_cube_caps_it_on_the_cutting_plane() {
    // A 20 cm cube cut by x = 0: the +X face goes, the −X face stays whole,
    // the four others are halved, and their cuts chain into the cap
    // (0, 10, −10), (0, 10, 10), (0, −10, 10), (0, −10, −10). The first
    // face (+X) has its last vertex outside, so the first cut, the +Y
    // face's (0, 10, −10) → (0, 10, 10), is taken as it is and the cap
    // winds counter-clockwise seen from +x.
    let mut volume = cube(10.0, FACES);
    volume.clip_by_plane(Vec3::X, 0.0);
    assert_eq!(volume.planes.len(), 6);
    for (plane, normal) in volume
        .planes
        .iter()
        .zip(FACES[1..].iter().chain([&Vec3::X]))
    {
        close_vec(plane.normal, *normal);
    }
    assert_eq!(volume.planes[0].vertices.len(), 4);
    for plane in &volume.planes[1..5] {
        assert_eq!(plane.vertices.len(), 4);
        assert!(plane.vertices.iter().all(|v| v.x <= 0.0));
    }
    let cap = &volume.planes[5];
    assert_eq!(cap.offset, 0.0);
    assert_eq!(
        cap.vertices,
        vec![
            Vec3::new(0.0, 10.0, -10.0),
            Vec3::new(0.0, 10.0, 10.0),
            Vec3::new(0.0, -10.0, 10.0),
            Vec3::new(0.0, -10.0, -10.0),
        ]
    );
    let mut refreshed = volume.clone();
    refreshed.refresh();
    close_vec(refreshed.planes[5].normal, Vec3::X);

    // Wholly inside: nothing changes. Wholly outside: nothing is left.
    let mut inside = cube(10.0, FACES);
    inside.clip_by_plane(Vec3::X, -20.0);
    assert_eq!(inside, cube(10.0, FACES));
    let mut outside = cube(10.0, FACES);
    outside.clip_by_plane(Vec3::X, 20.0);
    assert!(outside.planes.is_empty());
}

#[test]
fn a_first_face_inside_turns_the_first_cut_round() {
    // The same cut with the −X face first: its last vertex is inside, so
    // the first cut (still the +Y face's) is turned round, and the cap
    // winds the other way, as the original builds it.
    let order = [
        Vec3::NEG_X,
        Vec3::X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ];
    let mut volume = cube(10.0, order);
    volume.clip_by_plane(Vec3::X, 0.0);
    let cap = volume.planes.last().unwrap();
    assert_eq!(
        cap.vertices,
        vec![
            Vec3::new(0.0, 10.0, 10.0),
            Vec3::new(0.0, 10.0, -10.0),
            Vec3::new(0.0, -10.0, -10.0),
            Vec3::new(0.0, -10.0, 10.0),
        ]
    );
    let mut refreshed = volume.clone();
    refreshed.refresh();
    close_vec(refreshed.planes.last().unwrap().normal, Vec3::NEG_X);
}

#[test]
fn an_open_chain_is_extended_from_its_start() {
    // Three faces of an open box, cut by x = 0: the floor z = 0 (its last
    // vertex inside) cuts (0, 0, 0) → (0, 10, 0), turned round; the wall
    // y = 0 cuts (0, 0, 0) → (0, 0, 10) and the wall y = 10 cuts
    // (0, 10, 0) → (0, 10, 10). From (0, 10, 0), (0, 0, 0) the chain takes
    // (0, 0, 10); its ends are apart, so it is turned and extended by
    // (0, 10, 10), then turned back.
    let face = |vertices: [Vec3; 4]| Plane {
        normal: Vec3::Z,
        offset: 0.0,
        vertices: vertices.to_vec(),
    };
    let mut volume = ConvexVolume {
        planes: vec![
            face([
                Vec3::new(-10.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 10.0, 0.0),
                Vec3::new(-10.0, 10.0, 0.0),
            ]),
            face([
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(-10.0, 0.0, 0.0),
                Vec3::new(-10.0, 0.0, 10.0),
                Vec3::new(10.0, 0.0, 10.0),
            ]),
            face([
                Vec3::new(-10.0, 10.0, 0.0),
                Vec3::new(10.0, 10.0, 0.0),
                Vec3::new(10.0, 10.0, 10.0),
                Vec3::new(-10.0, 10.0, 10.0),
            ]),
        ],
    };
    volume.clip_by_plane(Vec3::X, 0.0);
    assert_eq!(volume.planes.len(), 4);
    assert_eq!(
        volume.planes[0].vertices,
        vec![
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(-10.0, 10.0, 0.0),
        ]
    );
    assert_eq!(
        volume.planes[3].vertices,
        vec![
            Vec3::new(0.0, 10.0, 10.0),
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 10.0),
        ]
    );
}

#[test]
fn a_shadow_volume_extrudes_the_silhouette_toward_the_light() {
    // Light travelling down (−y, w = 0): every face but −Y has
    // n · light ≤ 0 and is open. The −Y face shares an edge with each side
    // face, so the four side faces give side planes; +Y shares none.
    let volume = cube(10.0, FACES);
    let (shadow, sides) = volume.shadow_volume(Vec4::new(0.0, -1.0, 0.0, 0.0));
    assert_eq!(sides, 4);
    assert_eq!(shadow.planes.len(), 5);
    // The +X face's edge (10, −10, 10) → (10, −10, −10) is the −Y face's
    // (10, −10, −10) → (10, −10, 10) reversed: light × edge =
    // (0, −1, 0) × (0, 0, −20) = (20, 0, 0), offset −(n · (10, −10, −10)).
    let expected = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z];
    for (plane, normal) in shadow.planes.iter().zip(expected) {
        close_vec(plane.normal, normal);
        close(plane.offset, -10.0);
        assert_eq!(plane.vertices.len(), 4);
    }
    assert_eq!(
        shadow.planes[0].vertices,
        vec![
            Vec3::new(10.0, -10.0, 10.0),
            Vec3::new(10.0, -10.0, -10.0),
            Vec3::new(10.0, 9990.0, -10.0),
            Vec3::new(10.0, 9990.0, 10.0),
        ]
    );
    // The kept face closes the volume, without its polygon.
    let last = &shadow.planes[4];
    close_vec(last.normal, Vec3::NEG_Y);
    close(last.offset, -10.0);
    assert!(last.vertices.is_empty());
}

#[test]
fn a_shadow_volume_skips_edges_under_a_centimetre() {
    // A cube 0.8 cm across: every edge is shorter than 1 cm, so no side
    // plane; only the face away from the light is kept.
    let volume = cube(0.4, FACES);
    let (shadow, sides) = volume.shadow_volume(Vec4::new(0.0, -1.0, 0.0, 0.0));
    assert_eq!(sides, 0);
    assert_eq!(shadow.planes.len(), 1);
    close_vec(shadow.planes[0].normal, Vec3::NEG_Y);
}
