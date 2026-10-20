//! Tests of the chunk-tree writer: byte-exact chunks and round trips through
//! the reader, on synthetic scenes.
//! (Included from rfc_write.rs: `#[path = "rfc_write_test.rs"] mod tests;`)

use super::*;
use crate::characters::LevelCharacters;
use crate::rfc::{NodeBody, Scene};

const CM: f32 = 0.01;

fn name(s: &str) -> [u8; 16] {
    let mut n = [0u8; 16];
    n[..s.len()].copy_from_slice(s.as_bytes());
    n
}

fn at(t: [f32; 3]) -> Matrix {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], t]
}

fn node(n: &str, flags: u32, parent: Option<usize>) -> SceneNode {
    SceneNode {
        name: name(n),
        flags,
        parent,
        local: at([10.0, 20.0, 30.0]),
        world: at([100.0, 200.0, 300.0]),
        half_extents: [1.0, 2.0, 3.0],
        centre: [4.0, 5.0, 6.0],
        radius: 7.0,
        ..Default::default()
    }
}

fn plain(n: &str, parent: Option<usize>) -> SceneNode {
    node(n, class::PLAIN, parent)
}

fn export(scene: &ExportScene) -> Vec<u8> {
    export_with(scene, &ExportOptions::default())
}

fn export_with(scene: &ExportScene, options: &ExportOptions) -> Vec<u8> {
    Exporter::new().export(scene, options, None).unwrap()
}

fn scene_of(nodes: Vec<SceneNode>) -> ExportScene {
    ExportScene {
        nodes,
        whole_scene: true,
        ..Default::default()
    }
}

/// Top-level chunks: tag and payload.
fn roots(b: &[u8]) -> Vec<(u32, Vec<u8>)> {
    chunks(&b[4..])
}

fn chunks(mut b: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    while b.len() >= 8 {
        let t = u32::from_le_bytes(b[..4].try_into().unwrap());
        let n = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
        out.push((t, b[8..8 + n].to_vec()));
        b = &b[8 + n..];
    }
    out
}

fn words(b: &[u8]) -> Vec<u32> {
    b.as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_le_bytes(*w))
        .collect()
}

fn le(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A 1 m quad in the XZ plane: four positions, four UVs, two triangles in
/// two parts (the second without a material), one stream left out.
fn quad() -> ExportMesh {
    let positions = [
        0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 100.0, 0.0, 100.0, 0.0, 0.0, 100.0,
    ];
    let uvs = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
    ExportMesh {
        render_word: 0x0002_00FF,
        streams: vec![
            VertexStream {
                id: 1,
                format: 0x00F3_0001,
                count: 4,
                data: f32s(&positions),
            },
            VertexStream {
                id: 0x20,
                format: STREAM_UNSAVED | 0x00F3_0000,
                count: 4,
                data: f32s(&positions),
            },
            VertexStream {
                id: 0x10,
                format: 0x00F2_0010,
                count: 4,
                data: f32s(&uvs),
            },
        ],
        vertex_count: 4,
        parts: vec![
            MeshPart {
                vertices: [0, 3],
                triangles: [0, 0],
                material: Some(name("Wood")),
            },
            MeshPart {
                vertices: [0, 3],
                triangles: [1, 1],
                material: None,
            },
        ],
        maps: vec![
            IndexMap {
                key: 1,
                indices: vec![0, 1, 2, 3],
            },
            IndexMap {
                key: 0x10,
                indices: vec![0, 1, 2, 3],
            },
        ],
        triangles: vec![0, 1, 2, 0, 2, 3],
        triangle_bytes: Some(vec![7, 8]),
        triangle_flags: Some(vec![0x20_0000, 0x40_0000]),
        ..Default::default()
    }
}

fn mesh_node(n: &str, mesh: ExportMesh) -> SceneNode {
    SceneNode {
        body: NodeData::Mesh(Box::new(mesh)),
        ..node(n, flags::SAVED | class::MESH, None)
    }
}

#[test]
fn an_empty_scene_is_the_header_alone() {
    assert_eq!(export(&ExportScene::default()), [0xCF, 0xAF, 0x23, 0x3D]);
}

#[test]
fn chunks_store_the_bytes_after_their_size_word() {
    let mut w = ChunkWriter::new();
    w.open(0x10);
    w.u32(5);
    w.open(0x20);
    w.u16(1);
    w.close().unwrap();
    w.close().unwrap();
    assert_eq!(
        w.bytes_written(),
        [
            0x10, 0, 0, 0, 14, 0, 0, 0, 5, 0, 0, 0, 0x20, 0, 0, 0, 2, 0, 0, 0, 1, 0
        ]
    );
    w.open(0x30);
    w.u32(9);
    w.discard().unwrap();
    assert_eq!(w.position(), 22);
    assert!(w.close().is_err());
}

#[test]
fn the_grid_stores_its_sets_size_fixed_words_and_planes() {
    let grid = ExportGrid {
        wall_sets: vec![name("GEWS_UWA01")],
        floor_sets: vec![],
        width: 1,
        height: 1,
        plane_a: vec![0x8_0000],
        plane_b: vec![2],
        plane_c: vec![3],
    };
    let b = export(&ExportScene {
        grid: Some(grid),
        ..Default::default()
    });
    let r = roots(&b);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].0, tag::GRID);
    let mut want = le(&[1]);
    want.extend(name("GEWS_UWA01"));
    want.extend(le(&[0, 1, 1, 0, 600, 0x8_0000, 2, 3]));
    assert_eq!(r[0].1, want);
    let g = Scene::parse(&b).unwrap().grid.unwrap();
    assert_eq!(g.wall_sets, ["GEWS_UWA01"]);
    assert_eq!((g.width, g.height), (1, 1));
    assert_eq!((g.edges[0], g.plane_b[0], g.plane_c[0]), (0x8_0000, 2, 3));
}

#[test]
fn a_short_grid_plane_is_an_error() {
    let grid = ExportGrid {
        width: 2,
        height: 1,
        plane_a: vec![0, 0],
        plane_b: vec![0, 0],
        plane_c: vec![0],
        ..Default::default()
    };
    let scene = ExportScene {
        grid: Some(grid),
        ..Default::default()
    };
    assert!(
        Exporter::new()
            .export(&scene, &Default::default(), None)
            .is_err()
    );
}

#[test]
fn a_mesh_reads_back() {
    let b = export(&scene_of(vec![mesh_node("Quad", quad())]));
    let s = Scene::parse(&b).unwrap();
    assert_eq!(s.version, 0x3D23);
    assert_eq!(s.nodes.len(), 1);
    let n = &s.nodes[0];
    assert_eq!((n.kind, n.name.as_str()), (0x3D03, "Quad"));
    assert_eq!(n.transform[3], [10.0 * CM, 20.0 * CM, 30.0 * CM]);
    assert_eq!(n.bounds_half_extents, [1.0 * CM, 2.0 * CM, 3.0 * CM]);
    assert_eq!(n.bounds_centre, [4.0 * CM, 5.0 * CM, 6.0 * CM]);
    assert_eq!(n.bounds_radius, 7.0 * CM);
    let NodeBody::Mesh(m) = &n.body else {
        panic!("not a mesh: {:?}", n.body)
    };
    // The low byte of the render word is not stored.
    assert_eq!(m.render_flags, 0x0002_0000);
    assert_eq!(m.position_count, 4);
    assert_eq!(m.positions[2], [100.0 * CM, 0.0, 100.0 * CM]);
    assert_eq!(m.uvs.as_ref().unwrap()[2], [1.0, 1.0]);
    assert_eq!(m.indices, [0, 1, 2, 0, 2, 3]);
    assert_eq!(m.groups.len(), 2);
    assert_eq!(m.groups[0].material, "Wood");
    assert_eq!(m.groups[1].material, "_Default");
    assert_eq!((m.groups[1].first_index, m.groups[1].index_count), (3, 3));
    assert_eq!(
        m.triangle_flags.as_deref(),
        Some(&[0x20_0000, 0x40_0000][..])
    );
}

#[test]
fn the_stream_count_counts_only_saved_streams() {
    let mut w = ChunkWriter::new();
    write_mesh(&mut w, &quad(), &ExportOptions::default()).unwrap();
    let v = words(&w.bytes_written()[..12]);
    // Render word, streams − 1 (two of three saved), first stream's id.
    assert_eq!(v, [0x0002_0000, 1, 1]);
}

#[test]
fn indices_widen_from_65535_vertices() {
    let size = |vertex_count| {
        let mesh = ExportMesh {
            vertex_count,
            triangles: vec![0, 1, 2],
            ..Default::default()
        };
        let mut w = ChunkWriter::new();
        write_mesh(&mut w, &mesh, &ExportOptions::default()).unwrap();
        w.position()
    };
    // Three corners: 6 bytes narrow, 12 wide.
    assert_eq!(size(0xFFFF) - size(0xFFFE), 6);
}

#[test]
fn triangle_data_can_be_left_out() {
    let options = ExportOptions {
        skip_triangle_data: true,
        ..Default::default()
    };
    let mut full = ChunkWriter::new();
    write_mesh(&mut full, &quad(), &ExportOptions::default()).unwrap();
    let mut bare = ChunkWriter::new();
    write_mesh(&mut bare, &quad(), &options).unwrap();
    // 0x3DC2: 8 + 2 bytes; 0x3DC3: 8 + 8.
    assert_eq!(full.position() - bare.position(), 26);
}

#[test]
fn a_stream_shorter_than_its_count_is_an_error() {
    let mut mesh = quad();
    mesh.streams[0].data.truncate(40);
    let scene = scene_of(vec![mesh_node("Quad", mesh)]);
    assert!(
        Exporter::new()
            .export(&scene, &Default::default(), None)
            .is_err()
    );
}

#[test]
fn a_mesh_node_without_a_mesh_is_an_error() {
    let scene = scene_of(vec![node("Bad", flags::SAVED | class::MESH, None)]);
    assert!(
        Exporter::new()
            .export(&scene, &Default::default(), None)
            .is_err()
    );
}

#[test]
fn the_header_masks_both_flag_words() {
    let mut n = node("A", 0x8106_0001, None);
    n.flags2 = 0x2FF;
    n.tail = 0xABCD;
    let b = export(&scene_of(vec![n]));
    let r = roots(&b);
    assert_eq!(r[0].0, tag::NODE_LIST);
    let kids = chunks(&r[0].1[4..]);
    assert_eq!(kids[0].0, tag::NODE_PLAIN);
    let mut want = name("A").to_vec();
    want.extend(le(&[0x0000_0001, 0x270, 0, 0]));
    want.extend(f32s(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]));
    want.extend(f32s(&[10.0, 20.0, 30.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]));
    want.extend(le(&[0xABCD]));
    assert_eq!(kids[0].1, want);
}

#[test]
fn a_node_under_an_unexported_parent_stores_its_world_placement() {
    // A top-level group is walked through; its own node is not exported.
    let nodes = vec![node("Grp", class::GROUP, None), plain("Child", Some(0))];
    let s = Scene::parse(&export(&scene_of(nodes))).unwrap();
    assert_eq!(s.nodes.len(), 1);
    assert_eq!(s.nodes[0].parent, None);
    assert_eq!(
        s.nodes[0].transform[3],
        [100.0 * CM, 200.0 * CM, 300.0 * CM]
    );
}

#[test]
fn nodes_neither_saved_nor_plain_are_left_out_with_their_subtree() {
    let mut stale = node("Stale", 0x20 | flags::EXPORTED, None);
    stale.index = 9;
    let nodes = vec![stale, plain("Under", Some(0))];
    assert_eq!(export(&scene_of(nodes)), [0xCF, 0xAF, 0x23, 0x3D]);
}

#[test]
fn props_are_numbered_from_the_last_scene_node() {
    // Nodes 1 and 2, then the prop takes 2 + 1 − 1 + 1 = 3, which the node
    // under it (the third scene node) also holds.
    let nodes = vec![
        plain("A", None),
        plain("B", Some(0)),
        node("chair a01", flags::PROP | flags::SAVED, None),
        plain("N", Some(2)),
    ];
    let s = Scene::parse(&export(&scene_of(nodes))).unwrap();
    let parents: Vec<_> = s.nodes.iter().map(|n| n.parent).collect();
    assert_eq!(parents, [None, Some(1), Some(3)]);
    assert_eq!(s.props.len(), 1);
    assert_eq!(s.props[0].object, "chair a01");
    assert_eq!(s.props[0].parent, None);
}

#[test]
fn a_sharer_stores_only_the_lowest_numbered_source() {
    let track = ExportTrack {
        on_translation: true,
        mode: 1,
        duration: 3.0,
        keys: vec![(0.0, [0.0; 3])],
    };
    let mut x = plain("X", None);
    x.tracks = vec![track.clone()];
    let mut y = plain("Y", None);
    y.shares = Some(0);
    y.tracks = vec![track];
    let s = Scene::parse(&export(&scene_of(vec![x, y]))).unwrap();
    assert_eq!(s.nodes[0].key_tracks.len(), 1);
    assert!(matches!(s.nodes[1].body, NodeBody::Instance(1)));
    assert!(s.nodes[1].key_tracks.is_empty());
}

#[test]
fn materials_missing_from_the_shared_libraries_are_listed() {
    let mut a = quad();
    a.parts[1].material = Some(name("STONE"));
    let mut b = quad();
    b.parts[0].material = Some(name("WOOD"));
    b.parts[1].material = Some(name("Iron"));
    let mut payload = vec![0u8; 184];
    payload[0] = 0x5A;
    let scene = ExportScene {
        nodes: vec![mesh_node("A", a), mesh_node("B", b)],
        shared_materials: vec![name("stone")],
        material_library: vec![(name("Wood"), payload.clone())],
        ..Default::default()
    };
    let bytes = export(&scene);
    let r = roots(&bytes);
    assert_eq!(r[0].0, tag::MATERIAL_LIST);
    // wood and iron are listed; only wood is in the scene's library.
    assert_eq!(words(&r[0].1[..4]), [2]);
    let kids = chunks(&r[0].1[4..]);
    assert_eq!(kids.len(), 1);
    assert_eq!(kids[0].0, tag::MATERIAL);
    assert_eq!(&kids[0].1[..16], &name("wood"));
    assert_eq!(&kids[0].1[16..], &payload[..]);
}

/// A material payload with one texture layer naming `texture`.
fn layered(texture: &str, kind: u32) -> Vec<u8> {
    let mut p = vec![0u8; 184];
    let mut layer = vec![0u8; 56];
    layer[..4].copy_from_slice(&0x2_0000u32.to_le_bytes());
    layer[4..8].copy_from_slice(&56u32.to_le_bytes());
    layer[32..36].copy_from_slice(&kind.to_le_bytes());
    layer[40..56].copy_from_slice(&name(texture));
    p.extend(layer);
    p
}

fn written_payload(exporter: &mut Exporter, payload: Vec<u8>, options: &ExportOptions) -> Vec<u8> {
    let mut mesh = quad();
    mesh.parts[1].material = None;
    let scene = ExportScene {
        nodes: vec![mesh_node("A", mesh)],
        material_library: vec![(name("wood"), payload)],
        ..Default::default()
    };
    let b = exporter.export(&scene, options, None).unwrap();
    chunks(&roots(&b)[0].1[4..])[0].1[16..].to_vec()
}

#[test]
fn renamed_texture_layers_take_the_rfi_stem_and_kind_0x40() {
    let options = ExportOptions {
        rename_textures: true,
        ..Default::default()
    };
    let mut ex = Exporter::new();
    let p = written_payload(&mut ex, layered("Brick01.dds", 0x10), &options);
    assert_eq!(words(&p[184 + 32..184 + 36]), [0x40]);
    assert_eq!(&p[184 + 40..], &name("Brick01_rfi"));
    let want = TextureRename {
        source: name("brick01.rfi"),
        renamed: name("Brick01_rfi"),
    };
    assert_eq!(ex.texture_renames, std::slice::from_ref(&want));
    // A second export of the same texture adds no rename.
    written_payload(&mut ex, layered("Brick01.dds", 0x10), &options);
    assert_eq!(ex.texture_renames, [want]);
    // Other layer kinds are left alone.
    let p = written_payload(&mut ex, layered("Brick02.dds", 0x20), &options);
    assert_eq!(&p[184 + 40..], &name("Brick02.dds"));
}

#[test]
fn the_rfi_extension_keeps_the_layer_kind() {
    let options = ExportOptions {
        rfi_extension: true,
        ..Default::default()
    };
    let mut ex = Exporter::new();
    let p = written_payload(&mut ex, layered("Brick01.dds", 0x10), &options);
    assert_eq!(words(&p[184 + 32..184 + 36]), [0x10]);
    assert_eq!(&p[184 + 40..], &name("Brick01.rfi"));
    assert!(ex.texture_renames.is_empty());
}

#[test]
fn extensions_are_replaced_from_the_last_dot_or_the_end() {
    assert_eq!(replace_extension(&name("abc"), b"_rfi"), name("abc_rfi"));
    assert_eq!(replace_extension(&name("a.b.c"), b"_rfi"), name("a.b_rfi"));
    assert_eq!(
        replace_extension(&name("abcdefghijklmn"), b"_rfi"),
        name("abcdefghijklmn_r")
    );
    let full = name("abcdefghijklmnop");
    assert_eq!(replace_extension(&full, b"_rfi"), full);
    assert_eq!(lowercase_name(&name("AbZ.1[")), name("abz.1["));
}

#[test]
fn weights_turn_back_into_points_over_the_rest_points() {
    let rest = [
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [0.0, 20.0, 0.0],
        [0.0, 0.0, 40.0],
    ];
    assert_eq!(
        weighted_point([0.5, 0.25, 0.125, 0.125], &rest),
        [2.5, 2.5, 5.0]
    );
}

fn body() -> ExportBody {
    ExportBody {
        mobility: 1.5,
        rest: [
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 20.0, 0.0],
            [0.0, 0.0, 40.0],
        ],
        point_masses: [4.0; 4],
        friction: 0.75,
        group_mask: 3,
        last: 0,
        spheres: vec![[1.0, 2.0, 3.0, 4.0]],
        weighted_links: true,
        centre: [1.0, 1.0, 1.0],
        links: vec![
            ExportLink {
                flags: LINK_SLIDER,
                on_body: [0.5, 0.25, 0.125, 0.125],
                on_parent: [3.0, 4.0, 5.0, 0.0],
                slack: 49.0,
                strength: 1.0,
                smoothing: 0.5,
                axis: [0.0, 7.0, 0.0],
                ..Default::default()
            },
            ExportLink {
                flags: LINK_UNSAVED,
                ..Default::default()
            },
            ExportLink::default(),
        ],
        sounds: vec![ExportSound {
            id: 2,
            point: [0.0, 100.0, 0.0],
            sound: name("ChnAMov"),
            volume: 0.5,
            pitch: 1.0,
        }],
    }
}

#[test]
fn links_stop_at_the_first_unsaved_one() {
    let mut w = ChunkWriter::new();
    write_body(&mut w, ATTACH_BODY, &body()).unwrap();
    let b = w.into_bytes();
    // Header 92 bytes, spheres 8 + 24, then the links chunk.
    let links = &b[92 + 32..];
    assert_eq!(words(&links[..12]), [tag::LINKS, 4 + 4 + 24 + 12 + 12, 1]);
    // On body: the weights over the rest points; on parent: less the centre.
    assert_eq!(&links[16..40], &f32s(&[2.5, 2.5, 5.0, 2.0, 3.0, 4.0])[..]);
}

#[test]
fn a_rigid_body_reads_back() {
    let mut m = mesh_node("Door", quad());
    m.flags |= flags::ATTACHED;
    m.attachment = Some(Attachment {
        class: ATTACH_BODY | 0x400,
        id: 0,
        data: AttachmentData::Body(Box::new(body())),
    });
    let s = Scene::parse(&export(&scene_of(vec![m]))).unwrap();
    let r = s.nodes[0].rigid().unwrap();
    assert_eq!(r.class, 0x1000_0400);
    assert_eq!(r.mobility, 1.5);
    assert_eq!(r.rest[3], [0.0, 0.0, 40.0 * CM]);
    assert_eq!(r.spheres.len(), 1);
    assert_eq!(r.spheres[0].radius, 4.0 * CM);
    assert_eq!(r.links.len(), 1);
    assert_eq!(r.links[0].on_body, [2.5 * CM, 2.5 * CM, 5.0 * CM]);
    assert!(r.links[0].is_slider());
    assert_eq!(r.sounds[0].sound, "ChnAMov");
}

#[test]
fn an_attachment_is_written_only_under_its_node_flag() {
    let mut m = mesh_node("Hair", quad());
    m.attachment = Some(Attachment {
        class: ATTACH_CLOTH,
        id: 0,
        data: AttachmentData::Cloth {
            marker: 0,
            particles: vec![],
        },
    });
    let off = Scene::parse(&export(&scene_of(vec![m.clone()]))).unwrap();
    assert!(!off.nodes[0].cloth_marker);
    m.flags |= flags::ATTACHED;
    let on = Scene::parse(&export(&scene_of(vec![m]))).unwrap();
    assert!(on.nodes[0].cloth_marker);
}

#[test]
fn a_fire_reads_back_as_an_emitter() {
    let mut n = node("Fire", class::PLAIN | flags::ATTACHED, None);
    n.attachment = Some(Attachment {
        class: ATTACH_EMITTER,
        id: 0,
        data: AttachmentData::Emitter {
            kind: tag::FIRE,
            size: 2.5,
        },
    });
    let s = Scene::parse(&export(&scene_of(vec![n.clone()]))).unwrap();
    assert!(matches!(
        s.nodes[0].body,
        NodeBody::Emitter { kind: tag::FIRE, size } if size == 2.5
    ));
    n.attachment = Some(Attachment {
        class: ATTACH_EMITTER,
        id: 0,
        data: AttachmentData::Emitter {
            kind: 0x3DD0_E003,
            size: 1.0,
        },
    });
    assert!(matches!(
        Exporter::new().export(&scene_of(vec![n]), &Default::default(), None),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn a_light_body_is_40_bytes_plus_spot_and_extra() {
    let mut n = node("Omni", class::LIGHT | flags::SAVED, None);
    n.flags2 = SPOT;
    n.body = NodeData::Light(ExportLight {
        flags: 0x1_0010,
        color: [1.0, 0.5, 0.25],
        multiplier: 0.1,
        intensity: 60.0,
        radius: 630.0,
        unread: 0.0,
        softness: 0.5,
        spot: [2.0, 3.0],
        extra: [4.0, 5.0, 6.0],
    });
    let b = export(&scene_of(vec![n]));
    let kids = chunks(&roots(&b)[0].1[4..]);
    let body = &kids[0].1[112..];
    assert_eq!(body.len(), 40 + 8 + 16);
    assert_eq!(words(&body[16..20]), [1.0f32.to_bits()]);
    assert_eq!(words(&body[48..52]), [0]);
    let s = Scene::parse(&b).unwrap();
    let NodeBody::Light(l) = s.nodes[0].body else {
        panic!("not a light")
    };
    assert_eq!((l.flags, l.multiplier, l.intensity), (0x1_0010, 0.1, 60.0));
    assert_eq!((l.radius, l.softness), (630.0 * CM, 0.5));
    assert_eq!(l.spot, Some([2.0, 3.0]));
}

#[test]
fn a_voxel_light_reads_back() {
    let mut n = node("@Omni001", class::VOXEL_LIGHT | flags::SAVED, None);
    n.body = NodeData::VoxelLight(ExportVoxelLight {
        color: [0.216, 0.678, 1.0],
        intensity: 16.0,
        radius: 300.0,
    });
    let s = Scene::parse(&export(&scene_of(vec![n]))).unwrap();
    let NodeBody::VoxelLight(l) = s.nodes[0].body else {
        panic!("not a voxel light")
    };
    assert_eq!(l.color, [0.216, 0.678, 1.0]);
    assert_eq!((l.intensity, l.radius), (16.0, 300.0 * CM));
}

#[test]
fn a_target_node_stores_the_target_number() {
    let mut t = node("Cam", class::TARGET | flags::SAVED, None);
    t.body = NodeData::Target {
        node: 1,
        value: 0x3F80_0000,
    };
    let b = export(&scene_of(vec![t, plain("Aim", None)]));
    let kids = chunks(&roots(&b)[0].1[4..]);
    assert_eq!(kids[0].0, tag::NODE_TARGET);
    assert_eq!(words(&kids[0].1[112..]), [2, 0x3F80_0000]);
}

#[test]
fn key_tracks_and_gi_points_read_back() {
    let mut mesh = quad();
    mesh.gi_points = Some(GiPoints {
        spacing: 40,
        points: vec![[100.0, 0.0, 50.0]],
    });
    let mut m = mesh_node("Box001", mesh);
    m.tracks = vec![
        ExportTrack {
            on_translation: true,
            mode: 1,
            duration: 3.0,
            keys: vec![(0.0, [0.0; 3]), (1.0, [0.0, 0.0, -5.0])],
        },
        ExportTrack {
            on_translation: false,
            mode: 3,
            duration: 1.0,
            keys: vec![(0.0, [0.0; 3])],
        },
    ];
    let s = Scene::parse(&export(&scene_of(vec![m]))).unwrap();
    let n = &s.nodes[0];
    let tracks: Vec<_> = n.tracks().collect();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].duration, 3.0);
    assert_eq!(tracks[0].keys[1].position, [0.0, 0.0, -5.0 * CM]);
    let NodeBody::Mesh(m) = &n.body else {
        panic!("not a mesh")
    };
    assert_eq!(m.gi_points, [[100.0 * CM, 0.0, 50.0 * CM]]);
}

#[test]
fn a_track_record_is_byte_exact() {
    let mut w = ChunkWriter::new();
    let t = ExportTrack {
        on_translation: true,
        mode: 2,
        duration: 3.0,
        keys: vec![(1.0, [0.0, 0.0, -5.0])],
    };
    write_track(&mut w, &t).unwrap();
    let mut want = le(&[tag::KEY_TRACK, 32, tag::KEY_TRACK_WORD, 2]);
    want.extend(f32s(&[3.0]));
    want.extend(le(&[0]));
    want.extend(f32s(&[1.0, 0.0, 0.0, -5.0]));
    assert_eq!(w.into_bytes(), want);
}

fn owned(n: &str, parent: usize, class: u32) -> SceneNode {
    SceneNode {
        attachment: Some(Attachment {
            class,
            id: 0x77,
            data: if class == ATTACH_CLOTH {
                AttachmentData::Cloth {
                    marker: 0,
                    particles: vec![[1.0, 2.0, 3.0]],
                }
            } else {
                AttachmentData::None
            },
        }),
        ..node(n, flags::OWNED | class::MESH, Some(parent))
    }
}

#[test]
fn a_prop_stores_its_placement_and_part_records() {
    let prop = node("door uw01a", flags::PROP | flags::SAVED | 0x8_0234, None);
    let nodes = vec![
        prop,
        owned("Body", 0, ATTACH_BODY),
        owned("Stop", 0, ATTACH_EMITTER),
        owned("Cloth", 0, ATTACH_CLOTH),
    ];
    let b = export(&scene_of(nodes));
    let r = roots(&b);
    assert_eq!(r[0].0, tag::PROPS);
    let props = chunks(&r[0].1[4..]);
    let rec = &props[0].1;
    assert_eq!(words(&rec[..4]), [0x8_0000]);
    // An emitter part ends the records: only the body's is written.
    let parts = chunks(&rec[4 + 16 + 48 + 8..]);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].0, tag::PART_BODY);
    assert_eq!(&parts[0].1[..16], &name("Body"));
    assert_eq!(words(&parts[0].1[16..20]), [0x77]);
    assert_eq!(
        parts[0].1[20..],
        f32s(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 100.0, 200.0, 300.0
        ])
    );
    let s = Scene::parse(&b).unwrap();
    assert_eq!(s.props[0].flags, 0x8_0000);
    assert_eq!(s.props[0].transform[3], [10.0 * CM, 20.0 * CM, 30.0 * CM]);
    assert!(s.level_errors.is_empty());
}

#[test]
fn a_cloth_part_stores_its_particles() {
    let nodes = vec![
        node("banner", flags::PROP | flags::SAVED, None),
        owned("Cloth", 0, ATTACH_CLOTH),
    ];
    let b = export(&scene_of(nodes));
    let rec = &chunks(&roots(&b)[0].1[4..])[0].1;
    let parts = chunks(&rec[4 + 16 + 48 + 8..]);
    assert_eq!(parts[0].0, tag::PART_CLOTH);
    let mut want = name("Cloth").to_vec();
    want.extend(le(&[0x77, 1]));
    want.extend(f32s(&[1.0, 2.0, 3.0]));
    assert_eq!(parts[0].1, want);
}

#[test]
fn placed_objects_read_back_with_the_local_database() {
    let mut obj = node("Key", flags::OBJECT | flags::SAVED | 0x8_0000, Some(0));
    obj.object = Some(ObjectIds {
        id: 0x8000_0001,
        placement: 0x00EE_0000,
    });
    let scene = ExportScene {
        nodes: vec![plain("Room", None), obj],
        object_db: Some(RdbImage {
            count: 1,
            table: vec![0; 32],
            data: vec![],
        }),
        object_strings: vec![1, 2, 3, 4],
        ..Default::default()
    };
    let b = export(&scene);
    let r = roots(&b);
    let db = r.iter().find(|(t, _)| *t == tag::LOCAL_DATABASE).unwrap();
    let subs = chunks(&db.1);
    let tags: Vec<_> = subs.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        tags,
        [tag::LOCAL_OBJECTS, tag::LOCAL_STRINGS, tag::PLACED_OBJECTS]
    );
    assert_eq!(words(&subs[0].1[..8]), [OBJECT_DB_MAGIC, 32]);
    assert_eq!(subs[1].1, [1, 2, 3, 4]);
    let s = Scene::parse(&b).unwrap();
    assert!(s.level_errors.is_empty(), "{:?}", s.level_errors);
    assert_eq!(s.placed_objects.len(), 1);
    let o = &s.placed_objects[0];
    assert_eq!((o.id, o.flags, o.slot), (0x8000_0001, 0x00EE_0000, 1));
    assert_eq!(o.transform[3], [10.0 * CM, 20.0 * CM, 30.0 * CM]);
}

fn pose() -> PoseNode {
    let leaf = PoseNode {
        matrix: at([0.0, 50.0, 0.0]),
        class: POSE_CHILD_CLASS | 1,
        kind: POSE_LEAF_KIND,
        children: vec![PoseNode {
            class: POSE_CHILD_CLASS,
            ..Default::default()
        }],
        ..Default::default()
    };
    let other = PoseNode {
        class: 0x1300_0000,
        ..Default::default()
    };
    PoseNode {
        matrix: at([0.0, 100.0, 0.0]),
        word: 5,
        children: vec![leaf, other],
        ..Default::default()
    }
}

fn character(condition: i32) -> SceneNode {
    SceneNode {
        character: Some(ExportCharacter {
            record: 0x4000_0003,
            record_kind: 0x1000,
            body_position: [500.0, 90.0, -200.0],
            floor_height: 12.0,
            heading: 1.5,
            condition,
            motion_state: 6,
            motion_value: 0.25,
            pose: Some(pose()),
            parts: vec![at([1.0, 2.0, 3.0])],
            held: [None, Some(at([4.0, 5.0, 6.0]))],
            ai: Some(ExportAi {
                motile: true,
                skip_home: false,
                mode: 1,
                home: [7.0, 8.0, 9.0],
                home_heading: 0.5,
            }),
            ..Default::default()
        }),
        ..node("Bandit", flags::CHARACTER | flags::SAVED | 0x20, None)
    }
}

/// Sub-chunk tags of the first placement in a character section.
fn placement_subs(b: &[u8]) -> (Vec<u8>, Vec<u32>) {
    let r = roots(b);
    let section = &r.iter().find(|(t, _)| *t == tag::CHARACTERS).unwrap().1;
    let p = chunks(&section[4..])
        .into_iter()
        .find(|(t, _)| *t == tag::PLACEMENT)
        .unwrap()
        .1;
    let tags = chunks(&p[24..]).into_iter().map(|(t, _)| t).collect();
    (p, tags)
}

#[test]
fn a_dead_character_stores_its_pose_tree() {
    let b = export(&scene_of(vec![character(DEAD)]));
    let (p, tags) = placement_subs(&b);
    assert_eq!(words(&p[..4]), [0x4000_0003]);
    assert_eq!(p[4..16], f32s(&[500.0, 12.0, -200.0]));
    assert_eq!(tags, [tag::POSE_TREE]);
    let s = Scene::parse(&b).unwrap();
    let c = s.characters.unwrap();
    let pl = &c.placements[0];
    assert_eq!(pl.position, [5.0, 0.12, -2.0]);
    assert_eq!((pl.yaw, pl.condition), (1.5, DEAD));
    // The root and its one child of class 0x12, which as a leaf writes none.
    assert_eq!(pl.pose.len(), 2);
}

#[test]
fn a_living_character_stores_a_pose_only_when_posed() {
    let (_, tags) = placement_subs(&export(&scene_of(vec![character(0)])));
    assert!(tags.is_empty());
    let mut posed = character(0);
    posed.character.as_mut().unwrap().state_flags = STATE_POSED;
    let (_, tags) = placement_subs(&export(&scene_of(vec![posed])));
    assert_eq!(tags, [tag::POSE_TREE]);
}

#[test]
fn a_save_stores_the_ai_home_and_motion_state() {
    let save = ExportOptions {
        save_mode: true,
        ..Default::default()
    };
    let (p, tags) = placement_subs(&export_with(&scene_of(vec![character(0)]), &save));
    assert_eq!(tags, [tag::AI_HOME, tag::POSE_TREE, tag::MOTION]);
    let subs = chunks(&p[24..]);
    let mut home = f32s(&[7.0, 8.0, 9.0, 0.5]);
    home.extend(le(&[0]));
    assert_eq!(subs[0].1, home);
    let mut motion = le(&[6]);
    motion.extend(f32s(&[0.25]));
    motion.extend(le(&[0]));
    assert_eq!(subs[2].1, motion);
    // Another record kind keeps the home out.
    let mut other = character(0);
    other.character.as_mut().unwrap().record_kind = 0x1200;
    let (_, tags) = placement_subs(&export_with(&scene_of(vec![other]), &save));
    assert_eq!(tags, [tag::POSE_TREE, tag::MOTION]);
}

#[test]
fn a_class_one_character_stores_its_translation_and_tail() {
    let mut c = character(0);
    c.flags = flags::CHARACTER | class::PLAIN;
    c.character.as_mut().unwrap().tail = vec![9, 9];
    let b = export(&scene_of(vec![c]));
    let section = &roots(&b)[0].1;
    let p = &chunks(&section[4..])[0].1;
    let mut want = le(&[0x4000_0003]);
    want.extend(f32s(&[10.0, 20.0, 30.0, 1.5]));
    want.extend(le(&[0]));
    want.extend([9, 9]);
    assert_eq!(*p, want);
}

#[test]
fn the_character_database_precedes_the_placements() {
    let scene = ExportScene {
        character_db: Some(RdbImage {
            count: 1,
            table: vec![0; 32],
            data: vec![],
        }),
        ..scene_of(vec![character(DEAD)])
    };
    let b = export(&scene);
    let section = &roots(&b)[0].1;
    assert_eq!(words(&section[..4]), [1]);
    let subs = chunks(&section[4..]);
    assert_eq!(subs[0].0, tag::CHARACTER_DB);
    assert_eq!(words(&subs[0].1[..8]), [CHARACTER_DB_MAGIC, 32]);
    assert_eq!(subs[1].0, tag::PLACEMENT);
    let c = LevelCharacters::parse(section).unwrap();
    assert_eq!(c.placements.len(), 1);
}

#[test]
fn a_saved_dead_body_stores_its_parts_and_both_hands() {
    let c = character(DEAD).character.unwrap();
    let mut w = ChunkWriter::new();
    write_character_state(&mut w, &c, true).unwrap();
    let b = w.into_bytes();
    let p = &chunks(&b)[0].1;
    let subs = chunks(&p[24..]);
    assert_eq!(subs[0].0, tag::POSE_FLAT);
    assert_eq!(subs[0].1.len(), 3 * 48);
    let mut empty_hand = Vec::new();
    for row in IDENTITY {
        empty_hand.extend(f32s(&row));
    }
    assert_eq!(subs[0].1[48..96], empty_hand);
}

#[test]
fn attachments_that_write_nothing_leave_no_chunk() {
    let mut c = character(0).character.unwrap();
    c.ai.as_mut().unwrap().mode = 0;
    c.attachments = vec![None, None];
    let mut w = ChunkWriter::new();
    write_character_state(&mut w, &c, true).unwrap();
    let b = w.into_bytes();
    let tags: Vec<_> = chunks(&chunks(&b)[0].1[24..])
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    assert_eq!(tags, [tag::AI_HOME]);
    c.attachments = vec![Some(vec![1, 2, 3, 4]), None, Some(vec![5, 6, 7, 8])];
    let mut w = ChunkWriter::new();
    write_character_state(&mut w, &c, true).unwrap();
    let b = w.into_bytes();
    let subs = chunks(&chunks(&b)[0].1[24..]);
    assert_eq!(subs[1].0, tag::ATTACHED);
    assert_eq!(subs[1].1, [2, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn the_environment_is_written_for_the_whole_scene_unless_marked() {
    let mut env = vec![0u8; ENVIRONMENT_LEN];
    env[0] = 0x00;
    env[2] = 0x01;
    let scene = ExportScene {
        environment: Some(env.clone()),
        whole_scene: true,
        ..Default::default()
    };
    let b = export(&scene);
    assert_eq!(roots(&b), [(tag::ENVIRONMENT, env.clone())]);
    assert!(Scene::parse(&b).unwrap().environment.is_some());
    let part = ExportScene {
        whole_scene: false,
        ..scene.clone()
    };
    assert_eq!(export(&part).len(), 4);
    let mut marked = env.clone();
    marked[1] = 0x01;
    let marked = ExportScene {
        environment: Some(marked),
        ..scene.clone()
    };
    assert_eq!(export(&marked).len(), 4);
    let short = ExportScene {
        environment: Some(vec![0; 10]),
        ..scene
    };
    assert!(
        Exporter::new()
            .export(&short, &Default::default(), None)
            .is_err()
    );
}

#[test]
fn root_chunks_follow_the_original_order() {
    let mut obj = node("Key", flags::OBJECT | flags::SAVED, None);
    obj.object = Some(ObjectIds::default());
    let scene = ExportScene {
        nodes: vec![
            character(DEAD),
            obj,
            node("chair", flags::PROP | flags::SAVED, None),
            mesh_node("Quad", quad()),
        ],
        grid: Some(ExportGrid {
            width: 1,
            height: 1,
            plane_a: vec![0],
            plane_b: vec![0],
            plane_c: vec![0],
            ..Default::default()
        }),
        material_library: vec![(name("wood"), vec![0; 184])],
        environment: Some(vec![0; ENVIRONMENT_LEN]),
        whole_scene: true,
        ..Default::default()
    };
    let mut extra = |w: &mut ChunkWriter| {
        w.open(0x1234);
        w.close()
    };
    let b = Exporter::new()
        .export(&scene, &ExportOptions::default(), Some(&mut extra))
        .unwrap();
    let tags: Vec<_> = roots(&b).into_iter().map(|(t, _)| t).collect();
    assert_eq!(
        tags,
        [
            tag::GRID,
            tag::MATERIAL_LIST,
            tag::NODE_LIST,
            tag::PROPS,
            tag::LOCAL_DATABASE,
            tag::CHARACTERS,
            tag::ENVIRONMENT,
            0x1234
        ]
    );
    let s = Scene::parse(&b).unwrap();
    assert!(s.level_errors.is_empty(), "{:?}", s.level_errors);
    assert_eq!(s.nodes.len(), 1);
    assert_eq!(s.props.len(), 1);
    assert_eq!(s.placed_objects.len(), 1);
    assert_eq!(s.characters.unwrap().placements.len(), 1);
}

#[test]
fn a_parent_past_the_node_list_is_an_error() {
    let scene = scene_of(vec![plain("A", Some(4))]);
    assert!(
        Exporter::new()
            .export(&scene, &Default::default(), None)
            .is_err()
    );
}
