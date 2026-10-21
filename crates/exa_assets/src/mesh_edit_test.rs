use super::*;
use exa_formats::material::{TextureLayer, TextureSlot};

/// A unit square in the XZ plane facing +Y as two triangles, its texture
/// coordinates its x and −z, one smoothing key, one material group.
pub(crate) fn square() -> Mesh {
    let positions = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
    ];
    Mesh {
        normals: vec![[0.0, 1.0, 0.0]; 4],
        uvs: Some(positions.iter().map(|p| [p[0], -p[2]]).collect()),
        position_ids: vec![0, 1, 2, 3],
        position_count: 4,
        uv_ids: vec![0, 1, 2, 3],
        uv_count: 4,
        smoothing: vec![0; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
        groups: vec![MeshGroup {
            material: "mat".into(),
            first_index: 0,
            index_count: 6,
        }],
        positions,
        ..Mesh::default()
    }
}

fn corners(inc: &Incidence, v: usize) -> Vec<(u32, u8)> {
    inc.corners[inc.of(v)]
        .iter()
        .map(|c| (c.face, c.corner))
        .collect()
}

#[test]
fn an_element_lists_its_last_corner_first() {
    let inc = Incidence::build(&[[0, 1, 2], [0, 2, 3]], 4).unwrap();
    assert_eq!(inc.starts, [0, 2, 3, 5, 6]);
    assert_eq!(corners(&inc, 0), [(1, 0), (0, 0)]);
    assert_eq!(corners(&inc, 2), [(1, 1), (0, 2)]);
    assert!(Incidence::build(&[[0, 1, 4]], 4).is_err());
}

/// Four faces round one point with smoothing bits 1, 4, 1|2 and 2: the
/// seed (the last face, bits 2) grows to 3 through face 2, so faces 3, 2
/// and 0 smooth together and face 1 is alone.
#[test]
fn smoothing_bits_merge_through_shared_bits() {
    let faces = [[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]];
    let mut inc = Incidence::build(&faces, 5).unwrap();
    let groups = smoothing_groups(&mut inc, &[1, 4, 3, 2]);
    assert_eq!(&groups[..3], &[0, 3, 4]);
    assert_eq!(corners(&inc, 0), [(3, 0), (2, 0), (0, 0), (1, 0)]);
    // Point 1 (faces 3 and 0): bits 2 and 1 share none.
    assert_eq!(&groups[3..5], &[5, 6]);
}

#[test]
fn a_face_without_smoothing_bits_is_its_own_group() {
    let faces = [[0, 1, 2], [0, 2, 3]];
    let mut inc = Incidence::build(&faces, 4).unwrap();
    let groups = smoothing_groups(&mut inc, &[0, 0]);
    // Point 0 has two corners, each its own group.
    assert_eq!(&groups[..3], &[0, 1, 2]);
}

#[test]
fn runs_split_ranges_by_key_in_order_of_appearance() {
    let mut inc = Incidence {
        corners: (0..5).map(|f| Corner { face: f, corner: 0 }).collect(),
        starts: vec![0, 5],
    };
    let key = [4u32, 2, 4, 2, 8];
    let starts = inc.starts.clone();
    let runs = split_runs(&mut inc, &starts, |c| key[c.face as usize]);
    assert_eq!(runs, [(4, 0), (2, 2), (8, 4), (0, 5)]);
    let faces: Vec<u32> = inc.corners.iter().map(|c| c.face).collect();
    assert_eq!(faces, [0, 2, 1, 3, 4]);
    // Refining only the runs keyed 2 by face parity keeps the others.
    let refined = refine_runs(&mut inc, &runs, 2, |c| c.face & 1);
    assert_eq!(refined, [(4, 0), (2, 2), (8, 4), (0, 5)]);
    let refined = refine_runs(&mut inc, &runs, 4, |c| c.face);
    assert_eq!(refined, [(4, 0), (4, 1), (2, 2), (8, 4), (0, 5)]);
}

#[test]
fn textured_and_normal_mapped_materials_split_their_vertices() {
    let layer = |slot| TextureLayer {
        slot,
        texture: "t".into(),
        parameter: 0,
        amount: 1.0,
    };
    let mut m = Material::default();
    assert_eq!(material_split_flags(&m), 0);
    m.layers = vec![layer(TextureSlot::Reflection)];
    assert_eq!(material_split_flags(&m), 0);
    m.layers.push(layer(TextureSlot::Diffuse));
    assert_eq!(material_split_flags(&m), SPLIT_UVS);
    m.layers.push(layer(TextureSlot::Normal));
    assert_eq!(material_split_flags(&m), SPLIT_UVS | SPLIT_TANGENTS);
}

#[test]
fn the_editable_square_keeps_one_index_per_corner() {
    let edit = EditMesh::from_mesh(&square(), &[SPLIT_UVS]).unwrap();
    assert_eq!(edit.face_count(), 2);
    assert_eq!(edit.channels[0].corners, [[0, 1, 2], [0, 2, 3]]);
    assert_eq!(edit.channels[0].key, 0x0001_0001);
    assert_eq!(edit.channels[1].key & KEY_MASK, KEY_UVS);
    assert_eq!(edit.channels[2].key, KEY_SMOOTHING);
    assert_eq!(edit.streams[1].format, 0x00F2_0010);
    assert_eq!(edit.groups[0].flags, SPLIT_UVS);
    assert_eq!((edit.groups[0].first, edit.groups[0].end), (0, 2));
}

#[test]
fn rendering_the_square_shares_its_vertices() {
    let mut edit = EditMesh::from_mesh(&square(), &[SPLIT_UVS]).unwrap();
    let mesh = edit.render(true).unwrap();
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
    for n in &mesh.normals {
        assert!((n[1] - 1.0).abs() < 1e-6);
    }
    // Unified vertices are numbered by first use of the reordered faces.
    let firsts: Vec<u32> = mesh.indices.iter().copied().take(3).collect();
    assert_eq!(firsts, [0, 1, 2]);
    // Every corner still names the same position and coordinate.
    for (t, i) in mesh.indices.as_chunks::<3>().0.iter().zip(0..) {
        for &v in t {
            let p = mesh.positions[v as usize];
            assert_eq!(
                mesh.uvs.as_ref().unwrap()[v as usize],
                [p[0], -p[2]],
                "face {i}"
            );
        }
    }
}

/// A seam on the square's diagonal: the second face names other texture
/// elements there. A textured group splits the two diagonal points, an
/// untextured one does not.
#[test]
fn texture_seams_split_only_textured_groups() {
    let mut m = square();
    m.positions.extend([[1.0, 0.0, -1.0], [0.0, 0.0, 0.0]]);
    m.uvs.as_mut().unwrap().extend([[0.5, 0.5], [0.25, 0.25]]);
    m.position_ids.extend([2, 0]);
    m.uv_ids.extend([4, 5]);
    m.uv_count = 6;
    m.smoothing.extend([0, 0]);
    m.indices = vec![0, 1, 2, 5, 4, 3];
    let mut textured = EditMesh::from_mesh(&m, &[SPLIT_UVS]).unwrap();
    assert_eq!(textured.render(false).unwrap().positions.len(), 6);
    let mut plain = EditMesh::from_mesh(&m, &[0]).unwrap();
    assert_eq!(plain.render(false).unwrap().positions.len(), 4);
}

/// The second face's texture is mirrored: a normal-mapped group splits its
/// shared points, a merely textured one does not.
#[test]
fn mirrored_texture_splits_normal_mapped_groups() {
    let mut m = square();
    // Face 1 (0, 2, 3) maps 3 to (2, 0): clockwise in texture space.
    m.uvs.as_mut().unwrap()[3] = [2.0, 0.0];
    let mut plain = EditMesh::from_mesh(&m, &[SPLIT_UVS]).unwrap();
    assert_eq!(plain.render(false).unwrap().positions.len(), 4);
    let mut mapped = EditMesh::from_mesh(&m, &[SPLIT_UVS | SPLIT_TANGENTS]).unwrap();
    assert_eq!(mapped.render(false).unwrap().positions.len(), 6);
}

#[test]
fn smoothing_bits_split_the_normals() {
    // A roof: two faces folded along x = 0.5, different smoothing bits.
    let mut m = square();
    m.positions[1][1] = 0.5;
    m.positions[2][1] = 0.5;
    m.indices = vec![0, 1, 2, 0, 2, 3];
    m.triangle_flags = Some(vec![1, 2]);
    let mut edit = EditMesh::from_mesh(&m, &[0]).unwrap();
    let mesh = edit.render(false).unwrap();
    // Points 0 and 2 are on both faces: split, 6 vertices.
    assert_eq!(mesh.positions.len(), 6);
    // The smoothing channel now numbers the groups.
    assert_eq!(edit.channel_index(KEY_SMOOTHING), Some(2));
    let same = EditMesh::from_mesh(
        &Mesh {
            triangle_flags: Some(vec![1, 1]),
            ..m.clone()
        },
        &[0],
    )
    .unwrap()
    .render(false)
    .unwrap();
    assert_eq!(same.positions.len(), 4);
}

#[test]
fn faces_outside_every_group_cannot_be_rendered() {
    let mut m = square();
    m.groups[0].index_count = 3;
    let mut edit = EditMesh::from_mesh(&m, &[0]).unwrap();
    assert!(edit.render(false).is_err());
}

#[test]
fn bounds_floor_the_half_extents() {
    let (centre, half, radius) = bounds(&[[0.0, 1.0, 0.0], [2.0, 1.0, 0.0]]).unwrap();
    assert_eq!(centre, [1.0, 1.0, 0.0]);
    assert_eq!(half[0], 1.0);
    // 0.001 cm, in metres.
    assert_eq!(half[1], 0.001 * 0.01);
    assert_eq!(radius, 1.0);
    assert!(bounds(&[]).is_none());
}

#[test]
fn malformed_meshes_are_errors() {
    let mut m = square();
    m.indices.push(0);
    assert!(EditMesh::from_mesh(&m, &[]).is_err());
    let mut m = square();
    m.indices[0] = 9;
    assert!(EditMesh::from_mesh(&m, &[]).is_err());
    // A short colour stream sharing the positions' channel is allowed (it
    // takes their count after an edit); an interpolated one is read by
    // subdivision and must cover its corners.
    let mut edit = EditMesh::from_mesh(&square(), &[]).unwrap();
    edit.streams.push(Stream {
        id: SHARES_CHANNEL | KEY_POSITIONS,
        format: TYPE_RGBA8,
        channel: 0,
        data: StreamData::Rgba8(vec![[0; 4]; 1]),
    });
    assert!(edit.validate().is_ok());
    edit.streams.last_mut().unwrap().format |= KEY_UVS;
    assert!(edit.validate().is_err());
}
