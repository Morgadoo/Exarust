use super::*;
use crate::mesh_edit::Incidence;

/// Values of the original's fast logarithm and exponential, worked out
/// from its constants and operation order.
#[test]
fn the_fast_curves_match_the_original_bit_for_bit() {
    assert_eq!(fast_ln(1.0).to_bits(), 0x3780_0000);
    assert_eq!(fast_ln(2.0).to_bits(), 0x3F31_7300);
    assert_eq!(fast_ln(10.0).to_bits(), 0x4013_5DA0);
    assert_eq!(fast_exp(0.0).to_bits(), 0x3F80_0016);
    assert_eq!(fast_exp(1.0).to_bits(), 0x402D_F826);
    assert_eq!(fast_exp(-1.0).to_bits(), 0x3EBC_5A94);
    assert_eq!(fast_ln(0.0), f32::NEG_INFINITY);
}

#[test]
fn vertices_score_two_over_the_root_of_their_triangles() {
    assert_eq!(valence_score(0), 0.0);
    assert_eq!(valence_score(1).to_bits(), 0x3FFF_FF7C);
    assert_eq!(valence_score(2).to_bits(), 0x3FB5_04AC);
    assert_eq!(valence_score(4).to_bits(), 0x3F7F_FF7C);
    assert_eq!(valence_score(32).to_bits(), 0x3EB5_04AC);
    // Past the table the curve is worked out each time.
    assert_eq!(valence_score(33).to_bits(), 0x3EB2_41BF);
}

#[test]
fn cache_places_score_three_quarters_then_fall() {
    for k in 0..3 {
        assert_eq!(place_score(k), 0.75);
    }
    assert_eq!(place_score(3).to_bits(), 0x3F80_00C7);
    assert_eq!(place_score(4).to_bits(), 0x3F72_E03C);
    assert_eq!(place_score(31).to_bits(), 0x3BD1_D16C);
    assert_eq!(place_score(32), 0.0);
}

#[test]
fn the_list_sorts_highest_first_keeping_ties_in_order() {
    let score = [1.0, 3.0, 2.0, 3.0, 0.5];
    let mut links: Vec<(i32, i32)> = (0..5).map(|i| (i + 1, i - 1)).collect();
    links[4].0 = -1;
    let mut at = sort_list(0, &mut links, &score);
    let mut seen = Vec::new();
    let mut prev = -1;
    while at >= 0 {
        assert_eq!(links[at as usize].1, prev);
        seen.push(at);
        prev = at;
        at = links[at as usize].0;
    }
    assert_eq!(seen, [1, 3, 2, 0, 4]);
}

#[test]
fn the_flag_sort_orders_by_key() {
    let mut a = [5u32, 1, 4, 1, 3, 9, 2];
    sort_by_key(&mut a, 0, 6, &|x| x);
    assert_eq!(a, [1, 1, 2, 3, 4, 5, 9]);
    let mut b = [3u32, 1, 2];
    sort_by_key(&mut b, 1, 2, &|x| x);
    assert_eq!(b, [3, 1, 2]);
}

fn group(first: usize, end: usize) -> FaceGroup {
    FaceGroup {
        material: "m".into(),
        first,
        end,
        flags: 0,
    }
}

/// Faces C (5, 4, 6), B (3, 4, 5) and the lone triangle A (0, 1, 2): A
/// scores highest (three points of one triangle each) and goes first.
/// Nothing in the cache is left, so the list head follows: C, the earlier
/// of the tied C and B. B then scores the cache bonus and goes last.
#[test]
fn faces_are_drawn_greedily_and_vertices_renumbered_by_first_use() {
    let mut tri = vec![[5, 4, 6], [3, 4, 5], [0, 1, 2]];
    let mut inc = Incidence::build(&tri, 7).unwrap();
    let corners: Vec<(u32, u32)> = (0..7).map(|v| (inc.starts[v], inc.starts[v + 1])).collect();
    let mut flags = vec![10u32, 20, 30];
    let mut faces = FaceArrays {
        tri: &mut tri,
        flags: Some(&mut flags),
        edges: None,
        children: None,
    };
    let renumber = order(&mut faces, &mut inc, &corners, &[group(0, 3)], 0);
    assert_eq!(tri, [[0, 1, 2], [3, 4, 5], [6, 4, 3]]);
    assert_eq!(flags, [30, 10, 20]);
    assert_eq!(renumber, [0, 1, 2, 6, 4, 3, 5]);
    // The incidence names the faces' new places.
    let of_three: Vec<u32> = inc.corners[inc.of(3)].iter().map(|c| c.face).collect();
    assert_eq!(of_three, [2]);
}

#[test]
fn flagged_meshes_sort_each_group_by_face_flags() {
    let mut tri = vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]];
    let mut inc = Incidence::build(&tri, 9).unwrap();
    let corners: Vec<(u32, u32)> = (0..9).map(|v| (inc.starts[v], inc.starts[v + 1])).collect();
    // The top nibble is not part of the key.
    let mut flags = vec![0xF000_0002u32, 1, 0];
    let mut faces = FaceArrays {
        tri: &mut tri,
        flags: Some(&mut flags),
        edges: None,
        children: None,
    };
    order(
        &mut faces,
        &mut inc,
        &corners,
        &[group(0, 3)],
        SORT_BY_FLAGS,
    );
    assert_eq!(flags, [0, 1, 0xF000_0002]);
}

#[test]
fn a_single_face_keeps_its_order() {
    let mut tri = vec![[2, 0, 1]];
    let mut inc = Incidence::build(&tri, 3).unwrap();
    let corners: Vec<(u32, u32)> = (0..3).map(|v| (inc.starts[v], inc.starts[v + 1])).collect();
    let mut faces = FaceArrays {
        tri: &mut tri,
        flags: None,
        edges: None,
        children: None,
    };
    assert_eq!(
        order(&mut faces, &mut inc, &corners, &[group(0, 1)], 0),
        [0, 1, 2]
    );
    assert_eq!(tri, [[2, 0, 1]]);
}
