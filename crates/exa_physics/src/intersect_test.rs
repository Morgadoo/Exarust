//! Tests for the intersection library, pinned to numbers worked by hand
//! from the original's pseudo-code. Dimensions are the original's
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
    // On the segment.
    close(
        point_segment_distance_squared(Vec3::new(4.0, 0.0, 0.0), origin, along_x, 10.0),
        0.0,
    );
    // Exactly on the far end.
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
    // A point sphere on the face touches, one beyond it is apart.
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
    // Past the end, 2 above the end point, reach 2: touching.
    assert!(!sphere_segment_separated(
        &sweep,
        1.0,
        Vec3::new(12.0, 0.0, 0.0),
        1.0
    ));
    // Just above that.
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
    // Through the middle: entry 7/20, exit 13/20.
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
    // An end short of the inflated +x face: the sweep never reaches it.
    let ending_short = Sweep::new(Vec3::new(-10.0, 0.0, 0.0), Vec3::new(-4.0, 0.0, 0.0));
    assert!(!swept_sphere_aabb_hit(
        &ending_short,
        1.0,
        box_centre,
        extents
    ));
    // Grazing the inflated face at y = 3: the slab is closed, so it hits.
    let grazing = Sweep::new(Vec3::new(-10.0, 3.0, 0.0), Vec3::new(10.0, 3.0, 0.0));
    assert!(swept_sphere_aabb_hit(&grazing, 1.0, box_centre, extents));
    // One centimetre further out: the slabs do not overlap.
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

    // The same square turned 45°: its world y extent is 2√2 ≈ 2.828.
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
    assert!(segment_obb_hit(
        Vec3::new(-10.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        &obb
    ));
    assert!(!segment_obb_hit(
        Vec3::new(-10.0, 3.0, 0.0),
        Vec3::new(10.0, 3.0, 0.0),
        &obb
    ));
    // The entry exactly at the end of the segment is rejected.
    assert!(!segment_obb_hit(
        Vec3::new(-10.0, 0.0, 0.0),
        Vec3::new(-2.0, 0.0, 0.0),
        &obb
    ));
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
    // toward it.
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
}

#[test]
fn bounds_cover_every_point_with_a_floor() {
    let points = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(5.0, -2.0, 7.0)];
    let (centre, extents) = bounds_of_points(&points);
    assert_eq!(centre, Vec3::new(3.0, 0.0, 5.0));
    assert_eq!(extents, Vec3::new(2.0, 2.0, 2.0));
    // One point has no extent; the original floors it at 1e-4.
    let (centre, extents) = bounds_of_points(&[Vec3::splat(2.0)]);
    assert_eq!(centre, Vec3::splat(2.0));
    assert_eq!(extents, Vec3::splat(MIN_BOUNDS_EXTENT));
    // No points is Exarust's guard; the original assumes at least one.
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
fn the_cube_face_mask_keeps_the_faces_the_sphere_touches() {
    // Face bits: +X, −X, −Y, +Y, −Z, +Z.
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), 1.0),
        0x01
    );
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), 1.0),
        0x08
    );
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(0.0, -10.0, 0.0), 1.0),
        0x04
    );
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), 1.0),
        0x20
    );
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(0.0, 0.0, -10.0), 1.0),
        0x10
    );
    // Between +X and +Y.
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(10.0, 10.0, 0.0), 1.0),
        0x09
    );
    // A sphere touching the +X/+Y edge exactly keeps both faces.
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(10.0, 10.0, 0.0), 0.0),
        0x09
    );
    // A large sphere reaches every side.
    assert_eq!(
        cube_face_mask(Vec3::ZERO, Vec3::new(-10.0, 0.0, 0.0), 100.0),
        0x3f
    );
}

#[test]
fn a_volume_translates_its_planes_and_drops_the_polygons() {
    let mut volume = unit_box();
    volume.planes[0].vertices = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
    let moved = volume.translated(Vec3::new(2.0, 0.0, 0.0));
    // The +X face moves from x = 1 to x = 3.
    close(moved.planes[0].offset, 1.0);
    assert!(moved.planes[0].vertices.is_empty());
    // The −X face moves from x = −1 to x = −3.
    close(moved.planes[1].offset, -3.0);
    // The z faces are orthogonal to the shift and keep their offsets.
    close(moved.planes[4].offset, -1.0);
}

#[test]
fn refreshing_a_volume_rebuilds_its_planes_from_its_polygons() {
    let mut volume = ConvexVolume {
        planes: vec![Plane {
            normal: Vec3::ZERO,
            offset: 0.0,
            vertices: vec![
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
            ],
        }],
    };
    volume.refresh();
    close_vec(volume.planes[0].normal, Vec3::Z);
    close(volume.planes[0].offset, -1.0);
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
    // The square at z = 1 lies on the +Z face, so no plane cuts it.
    let polygon = [
        Vec3::new(-0.5, -0.5, 1.0),
        Vec3::new(0.5, -0.5, 1.0),
        Vec3::new(0.5, 0.5, 1.0),
        Vec3::new(-0.5, 0.5, 1.0),
    ];
    let apex = Vec3::ZERO;
    let pyramid = volume.clip_polygon(&polygon, apex).unwrap();
    assert_eq!(pyramid.planes.len(), 4);
    let root = 1.0 / 5.0f32.sqrt();
    let normals = [
        Vec3::new(2.0 * root, 0.0, root),
        Vec3::new(0.0, 2.0 * root, root),
        Vec3::new(-2.0 * root, 0.0, root),
        Vec3::new(0.0, -2.0 * root, root),
    ];
    for (i, plane) in pyramid.planes.iter().enumerate() {
        close_vec(plane.normal, normals[i]);
        close(plane.offset, 0.0);
        assert_eq!(plane.vertices.len(), 3);
        assert_eq!(plane.vertices[1], apex);
        assert_eq!(plane.vertices[2], polygon[i]);
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
