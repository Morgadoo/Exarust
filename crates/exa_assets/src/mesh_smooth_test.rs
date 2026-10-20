use super::*;
use crate::mesh_edit::SPLIT_UVS;
use exa_formats::rfc::{Mesh, MeshGroup};

/// A unit square in the XZ plane facing +Y as two triangles (0, 1, 2) and
/// (0, 2, 3), its texture coordinates its x and −z, one material group.
fn square() -> Mesh {
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

fn points(mesh: &EditMesh) -> &[[f32; 3]] {
    match &mesh.streams[0].data {
        StreamData::Vec3(p) => p,
        _ => panic!("positions"),
    }
}

fn uvs(mesh: &EditMesh) -> &[[f32; 2]] {
    match &mesh.streams[1].data {
        StreamData::Vec2(p) => p,
        _ => panic!("texture coordinates"),
    }
}

fn close2(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

/// The square's border is all creases: its edges get their midpoints and
/// its corners (two creases each) go halfway to the mean of their two
/// border midpoints. The diagonal is smooth: 3/8 of each end and 1/8 of
/// each opposite corner.
#[test]
fn the_square_subdivides_with_border_creases() {
    let mut mesh = EditMesh::from_mesh(&square(), &[SPLIT_UVS]).unwrap();
    let done = subdivide(&mut mesh, 0.0).unwrap();
    assert_eq!(
        done,
        Subdivided::Done {
            degenerate_faces: 0
        }
    );
    assert_eq!(
        points(&mesh),
        &[
            [0.125, 0.0, -0.125],
            [0.875, 0.0, -0.125],
            [0.875, 0.0, -0.875],
            [0.125, 0.0, -0.875],
            // The diagonal (made first, by face 0), then the borders.
            [0.5, 0.0, -0.5],
            [0.5, 0.0, 0.0],
            [1.0, 0.0, -0.5],
            [0.0, 0.0, -0.5],
            [0.5, 0.0, -1.0],
        ]
    );
    // Corner faces, then the middle one, face by face.
    assert_eq!(
        mesh.channels[0].corners,
        [
            [0, 5, 4],
            [5, 1, 6],
            [4, 6, 2],
            [4, 5, 6],
            [0, 4, 7],
            [4, 2, 8],
            [7, 8, 3],
            [7, 4, 8],
        ]
    );
    assert_eq!((mesh.groups[0].first, mesh.groups[0].end), (0, 8));
    assert_eq!(mesh.face_children.as_deref(), Some(&[3u32, 7][..]));
    assert_eq!(mesh.level, 1);
    // Texture coordinates follow the same rules in their own island.
    let expected = [
        [0.125, 0.125],
        [0.875, 0.125],
        [0.875, 0.875],
        [0.125, 0.875],
        [0.5, 0.5],
        [0.5, 0.0],
        [1.0, 0.5],
        [0.0, 0.5],
        [0.5, 1.0],
    ];
    assert_eq!(uvs(&mesh).len(), 9);
    for (u, e) in uvs(&mesh).iter().zip(expected) {
        assert!(close2(*u, e), "{u:?} {e:?}");
    }
    assert_eq!(mesh.channels[1].corners, mesh.channels[0].corners);
}

/// A closed tetrahedron has no creases: an edge point is 3/8 of each end
/// and 1/8 of each opposite point; a point with three neighbours keeps
/// 1 − 3/16 of itself and takes 1/16 of each.
#[test]
fn a_closed_surface_smooths_everywhere() {
    let positions = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    let m = Mesh {
        normals: vec![[0.0, 1.0, 0.0]; 4],
        position_ids: vec![0, 1, 2, 3],
        position_count: 4,
        smoothing: vec![0; 4],
        indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
        groups: vec![MeshGroup {
            material: "mat".into(),
            first_index: 0,
            index_count: 12,
        }],
        positions,
        ..Mesh::default()
    };
    let mut mesh = EditMesh::from_mesh(&m, &[0]).unwrap();
    subdivide(&mut mesh, 0.0).unwrap();
    let p = points(&mesh);
    assert_eq!(p.len(), 10);
    assert_eq!(p[0], [0.0625, 0.0625, 0.0625]);
    assert_eq!(p[1], [0.8125, 0.0625, 0.0625]);
    // Face 0's first edge runs from point 1 to point 0.
    assert_eq!(p[4], [0.375, 0.125, 0.125]);
    assert_eq!(mesh.face_count(), 16);
}

/// The second face's corner 3 is mapped below the diagonal, on the same
/// side as corner 1: the texture folds over the diagonal, which becomes a
/// seam taking the mean opposite coordinate projected onto it.
#[test]
fn a_texture_fold_becomes_a_seam() {
    let mut m = square();
    m.uvs.as_mut().unwrap()[3] = [0.6, 0.0];
    let mut mesh = EditMesh::from_mesh(&m, &[SPLIT_UVS]).unwrap();
    subdivide(&mut mesh, 0.0).unwrap();
    let uv = uvs(&mesh);
    // 3/8 of each end plus 1/4 of the projected mean (0.4, 0.4).
    assert!(close2(uv[4], [0.475, 0.475]), "{:?}", uv[4]);
    // Point 0 now lies on three seams and keeps its coordinate.
    assert_eq!(uv[0], [0.0, 0.0]);
}

/// Faces naming different coordinates at the diagonal get one edge point
/// each, both midpoints.
#[test]
fn a_texture_seam_gets_two_edge_points() {
    let mut m = square();
    m.positions.extend([[1.0, 0.0, -1.0], [0.0, 0.0, 0.0]]);
    m.normals.extend([[0.0, 1.0, 0.0]; 2]);
    m.uvs.as_mut().unwrap().extend([[2.0, 2.0], [3.0, 3.0]]);
    m.position_ids.extend([2, 0]);
    m.uv_ids.extend([4, 5]);
    m.uv_count = 6;
    m.smoothing.extend([0, 0]);
    m.indices = vec![0, 1, 2, 5, 4, 3];
    let mut mesh = EditMesh::from_mesh(&m, &[SPLIT_UVS]).unwrap();
    subdivide(&mut mesh, 0.0).unwrap();
    // The positions still share the diagonal's point.
    assert_eq!(points(&mesh).len(), 9);
    let uv = uvs(&mesh);
    assert_eq!(uv.len(), 12);
    // Face 0's diagonal point (its first edge) is the midpoint of (1, 1)
    // and (0, 0); face 1's (its second edge, after its border edge) of
    // (3, 3) and (2, 2).
    assert_eq!(uv[6], [0.5, 0.5]);
    assert_eq!(uv[10], [2.5, 2.5]);
}

#[test]
fn face_bits_and_colours_follow_the_split() {
    let mut m = square();
    m.triangle_flags = Some(vec![0x0001_0001, 0x0001_0001]);
    m.triangle_edges = Some(vec![6, 3]);
    m.colors = Some(vec![[10, 20, 30, 40]; 4]);
    let mut mesh = EditMesh::from_mesh(&m, &[SPLIT_UVS]).unwrap();
    subdivide(&mut mesh, 0.0).unwrap();
    assert_eq!(mesh.face_flags.as_deref(), Some(&[0x0001_0001u32; 8][..]));
    // The middle face's edge bits are the old ones turned by an edge.
    assert_eq!(
        mesh.face_edges.as_deref(),
        Some(&[6u8, 6, 6, 5, 3, 3, 3, 6][..])
    );
    let colour = mesh
        .streams
        .iter()
        .find_map(|s| match &s.data {
            StreamData::Rgba8(c) => Some(c.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(colour, vec![[0xFF; 4]; 9]);
    // One smoothing group from the face bits: the same smooth diagonal.
    assert_eq!(points(&mesh)[4], [0.5, 0.0, -0.5]);
}

#[test]
fn different_smoothing_groups_crease_the_diagonal() {
    let mut m = square();
    m.triangle_flags = Some(vec![1, 2]);
    let mut mesh = EditMesh::from_mesh(&m, &[SPLIT_UVS]).unwrap();
    subdivide(&mut mesh, 0.0).unwrap();
    // A crease diagonal gets its midpoint; points 0 and 2 now lie on three
    // creases and stay.
    let p = points(&mesh);
    assert_eq!(p[4], [0.5, 0.0, -0.5]);
    assert_eq!(p[0], [0.0, 0.0, 0.0]);
    assert_eq!(p[2], [1.0, 0.0, -1.0]);
    assert_eq!(p[1], [0.875, 0.0, -0.125]);
}

#[test]
fn too_many_faces_are_left_alone() {
    let faces = MAX_FACES / 4 + 1;
    let m = Mesh {
        positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
        normals: vec![[0.0, 1.0, 0.0]; 3],
        position_ids: vec![0, 1, 2],
        position_count: 3,
        smoothing: vec![0; 3],
        indices: [0, 1, 2].repeat(faces),
        groups: vec![MeshGroup {
            material: "mat".into(),
            first_index: 0,
            index_count: (faces * 3) as u32,
        }],
        ..Mesh::default()
    };
    let mut mesh = EditMesh::from_mesh(&m, &[0]).unwrap();
    let before = mesh.clone();
    assert_eq!(subdivide(&mut mesh, 0.0).unwrap(), Subdivided::TooManyFaces);
    assert_eq!(mesh, before);
}

#[test]
fn the_smoothed_square_renders() {
    let mut mesh = EditMesh::from_mesh(&square(), &[SPLIT_UVS]).unwrap();
    subdivide(&mut mesh, 0.0).unwrap();
    let out = mesh.render(true).unwrap();
    assert_eq!(out.indices.len(), 24);
    assert_eq!(out.positions.len(), 9);
    assert_eq!(out.groups[0].index_count, 24);
    for n in &out.normals {
        assert!((n[1] - 1.0).abs() < 1e-6);
    }
    // The edit mesh can be smoothed again from where rendering left it.
    assert!(matches!(
        subdivide(&mut mesh, 0.0).unwrap(),
        Subdivided::Done { .. }
    ));
    assert_eq!(mesh.face_count(), 32);
}

/// Every hair of `CharExt.rpk` (what the character-creation scene smooths)
/// smooths: four faces per face, finite positions and coordinates, every
/// triangle within the vertices (run with `cargo test -p exa_assets --
/// --ignored` and the game installed).
#[test]
#[ignore]
fn shipped_hair_smooths() {
    use exa_formats::rfc::{NodeBody, Scene};
    use exa_rpk::{GameData, PackKind};
    let Some(dir) = crate::locate_install(None) else {
        return;
    };
    let game = GameData::open(dir).unwrap();
    let pack = game.pack(PackKind::CharExt).unwrap();
    let mut meshes = 0;
    for e in pack.entries() {
        let scene = Scene::parse(pack.bytes(e)).unwrap();
        for node in &scene.nodes {
            let NodeBody::Mesh(mesh) = &node.body else {
                continue;
            };
            let flags = vec![SPLIT_UVS; mesh.groups.len()];
            let smoothed = smooth_mesh(mesh, &flags).unwrap().unwrap();
            let out = &smoothed.mesh;
            assert_eq!(out.indices.len(), mesh.indices.len() * 4, "{}", node.name);
            assert!(
                out.indices
                    .iter()
                    .all(|&i| (i as usize) < out.positions.len())
            );
            assert!(out.positions.iter().flatten().all(|v| v.is_finite()));
            assert!(out.uvs.iter().flatten().flatten().all(|v| v.is_finite()));
            assert_eq!(smoothed.degenerate_faces, 0, "{}", node.name);
            meshes += 1;
        }
    }
    assert_eq!(meshes, 24);
}
