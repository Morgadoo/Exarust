use exa_formats::rfc::{Mesh, MeshGroup, SkinWeights};

use super::*;

/// A mesh over `positions` with one triangle list naming vertices directly.
fn mesh(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
    let n = positions.len();
    Mesh {
        normals: vec![[0.0, 1.0, 0.0]; n],
        position_ids: (0..n as u32).collect(),
        position_count: n,
        indices,
        groups: vec![MeshGroup {
            material: "mat".into(),
            first_index: 0,
            index_count: 0,
        }],
        positions,
        ..Mesh::default()
    }
}

fn edit(mut m: Mesh) -> EditMesh {
    m.groups[0].index_count = m.indices.len() as u32;
    EditMesh::from_mesh(&m, &[0]).unwrap()
}

fn ring(mesh: &EditMesh, v: usize) -> Vec<(u32, u8)> {
    mesh.incidence.as_ref().unwrap().corners[mesh.incidence.as_ref().unwrap().of(v)]
        .iter()
        .map(|c| (c.face, c.corner))
        .collect()
}

#[test]
fn stream_element_sizes_follow_the_format_word() {
    assert_eq!(stream_element_size(0x00F3_0001), 12);
    assert_eq!(stream_element_size(0x00F2_0010), 8);
    assert_eq!(stream_element_size(0x02B4_0100), 4);
    assert_eq!(stream_element_size(0x02A4_1000), 32);
    assert_eq!(stream_element_size(0x0000_0000), 0);
}

#[test]
fn positions_within_a_hundredth_of_a_centimetre_weld() {
    // 0.009 cm apart: welded.
    let mut near = edit(mesh(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.00009, 0.0, 0.0]],
        vec![0, 1, 2],
    ));
    near.weld().unwrap();
    assert_eq!(near.streams[0].data.len(), 2);
    // 0.011 cm apart: kept.
    let mut far = edit(mesh(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.00011, 0.0, 0.0]],
        vec![0, 1, 2],
    ));
    far.weld().unwrap();
    assert_eq!(far.streams[0].data.len(), 3);
}

#[test]
fn a_welded_rings_corners_are_spliced_in_the_originals_order() {
    // Position 0 is shared by faces 0, 1 and 3, and position 1 (a duplicate)
    // by face 2. The survivor's corners run f3, f1, then the merged f2,
    // then f0: the ring keeps its head last.
    let mut positions = vec![[0.0, 0.0, 0.0], [0.00005, 0.0, 0.0]];
    for i in 0..10 {
        positions.push([10.0 + i as f32, 0.0, 0.0]);
    }
    let mut m = edit(mesh(positions, vec![0, 4, 5, 0, 6, 7, 1, 8, 9, 0, 10, 11]));
    m.weld().unwrap();
    assert_eq!(ring(&m, 0), [(3, 0), (1, 0), (2, 0), (0, 0)]);
    // The dead position is gone and the rest renumber in old order.
    assert_eq!(m.streams[0].data.len(), 9);
    assert_eq!(m.channels[0].corners[2], [0, 5, 6]);
}

#[test]
fn texture_coordinates_weld_at_their_own_tolerance() {
    let mut m = mesh(vec![[0.0; 3], [1.0; 3], [2.0; 3]], vec![0, 1, 2]);
    m.uvs = Some(vec![[0.0, 0.0], [0.0, 0.0], [1.0, 0.0]]);
    m.uv_ids = vec![0, 1, 2];
    m.uv_count = 3;
    let mut edit = edit(m);
    edit.weld().unwrap();
    assert_eq!(edit.streams[0].data.len(), 3);
    assert_eq!(edit.streams[1].data.len(), 2);
    // The sort's order makes element 1 the survivor of the two equal
    // coordinates (the quicksort swaps equals the same way the original's
    // does); both corners then name the first compacted element.
    assert_eq!(edit.channels[1].corners, [[0, 0, 1]]);
}

#[test]
fn shared_skin_weights_fold_into_their_survivor_and_normalise() {
    let mut m = mesh(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.00005, 0.0, 0.0],
            [0.0, 0.0, -1.0],
        ],
        vec![0, 1, 2, 0, 2, 3],
    );
    let w = |bone, weight| SkinWeights {
        bones: [bone, 0, 0, 0],
        weights: [weight, 0.0, 0.0, 0.0],
    };
    m.skin = Some(vec![w(1, 1.0), w(3, 1.0), w(2, 0.5), w(3, 1.0)]);
    let mut edit = edit(m);
    edit.weld_influences().unwrap();
    assert_eq!(edit.streams[0].data.len(), 3);
    let StreamData::Skin(weights) = &edit.streams[1].data else {
        panic!("skin stream expected");
    };
    assert_eq!(weights.len(), 3);
    assert_eq!(weights[0].bones[..2], [1, 2]);
    assert!((weights[0].weights[0] - 2.0 / 3.0).abs() < 1e-6);
    assert!((weights[0].weights[1] - 1.0 / 3.0).abs() < 1e-6);
    assert_eq!(weights[1].bones[0], 3);
    // The dropped position's own slot is the third old influence.
    assert_eq!(weights[2].bones[0], 3);
}

#[test]
fn welding_without_influences_dropped_the_shared_weights() {
    let mut m = mesh(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.00005, 0.0, 0.0],
            [0.0, 0.0, -1.0],
        ],
        vec![0, 1, 2, 0, 2, 3],
    );
    let w = |bone, weight| SkinWeights {
        bones: [bone, 0, 0, 0],
        weights: [weight, 0.0, 0.0, 0.0],
    };
    m.skin = Some(vec![w(1, 1.0), w(3, 1.0), w(2, 0.5), w(3, 1.0)]);
    let mut edit = edit(m);
    edit.weld().unwrap();
    let StreamData::Skin(weights) = &edit.streams[1].data else {
        panic!("skin stream expected");
    };
    assert_eq!(weights[0].bones, [1, 0, 0, 0]);
    assert_eq!(weights[0].weights, [1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn flipping_winding_swaps_the_first_and_third_element_of_every_channel() {
    let mut m = mesh(vec![[0.0; 3], [1.0; 3], [2.0; 3]], vec![0, 1, 2]);
    m.smoothing = vec![0, 1, 2];
    let mut edit = edit(m);
    edit.flip_winding();
    assert_eq!(edit.channels[0].corners, [[2, 1, 0]]);
    assert_eq!(edit.channels[1].corners, [[2, 1, 0]]);
    // The stream values keep their order.
    assert_eq!(edit.streams[0].data.len(), 3);
    assert_eq!(edit.channels[1].corners[0][0], 2);
}

#[test]
fn the_indirect_incidence_maps_each_corner_through_its_reference() {
    let refs = [[2, 1, 0]];
    let elements_at = [5, 6, 7];
    let inc = Incidence::build_indirect(&refs, &elements_at, 8).unwrap();
    let listed = |v: usize| -> Vec<(u32, u8)> {
        inc.corners[inc.of(v)]
            .iter()
            .map(|c| (c.face, c.corner))
            .collect()
    };
    assert_eq!(listed(7), [(0, 0)]);
    assert_eq!(listed(6), [(0, 1)]);
    assert_eq!(listed(5), [(0, 2)]);
    assert!(Incidence::build_indirect(&[[3, 0, 0]], &[9], 8).is_err());
}

#[test]
fn rebuilding_the_incidence_lists_each_positions_corners_last_first() {
    let mut edit = edit(mesh(
        vec![[0.0; 3], [1.0; 3], [2.0; 3], [3.0; 3]],
        vec![0, 1, 2, 0, 2, 3],
    ));
    edit.incidence = None;
    edit.rebuild_incidence().unwrap();
    assert_eq!(ring(&edit, 0), [(1, 0), (0, 0)]);
    assert_eq!(ring(&edit, 2), [(1, 1), (0, 2)]);
}
