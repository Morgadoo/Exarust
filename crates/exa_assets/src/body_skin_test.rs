use super::*;
use exa_formats::rfc::{Mesh, MeshGroup, Node};

fn part(name: &str, parent: Option<usize>, at: [f32; 3], positions: &[[f32; 3]]) -> Node {
    let n = positions.len();
    Node {
        kind: 0x3D03,
        name: name.into(),
        flags: 0x89,
        flags2: 0,
        parent,
        transform: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], at],
        bounds_centre: [0.0; 3],
        bounds_half_extents: [0.0; 3],
        bounds_radius: 0.0,
        rigid_body: None,
        cloth_marker: false,
        key_tracks: Vec::new(),
        morph: None,
        instance: None,
        unknown_sub_chunks: Vec::new(),
        body: NodeBody::Mesh(Box::new(Mesh {
            positions: positions.to_vec(),
            normals: vec![[0.0, 1.0, 0.0]; n],
            position_ids: (0..n as u32).collect(),
            position_count: n,
            smoothing: vec![0; n],
            indices: (0..n as u32).collect(),
            triangle_flags: Some(vec![0; n / 3]),
            groups: vec![MeshGroup {
                material: "skin".into(),
                first_index: 0,
                index_count: n as u32,
            }],
            ..Default::default()
        })),
    }
}

fn scene(nodes: Vec<Node>) -> Scene {
    Scene {
        version: 0,
        nodes,
        materials: Vec::new(),
        grid: None,
        props: Vec::new(),
        environment: None,
        characters: None,
        local_items: Vec::new(),
        local_records: Vec::new(),
        local_database: None,
        local_strings: None,
        local_string_error: None,
        placed_objects: Vec::new(),
        unknown_chunks: Vec::new(),
        level_errors: Vec::new(),
    }
}

#[test]
fn a_rigid_vertex_has_one_slot_like_the_originals_record() {
    let rigid = [(4, 1.0), (4, 0.0), (4, 0.0), (4, 0.0)];
    let r = record_of(&rigid);
    assert_eq!(r.bones, [4, 0, 0, 0]);
    assert_eq!(r.weights, [1.0, 0.0, 0.0, 0.0]);
    let split = [(4, 0.75), (5, 0.25), (0, 0.0), (0, 0.0)];
    assert_eq!(record_of(&split).bones, [4, 5, 0, 0]);
    assert_eq!(vertex_of(&record_of(&split)), split);
    // A group position whose share is 0 keeps its neighbour's slot.
    let unshared = [(4, 1.0), (5, 0.0), (0, 0.0), (0, 0.0)];
    assert_eq!(record_of(&unshared).bones, [4, 5, 0, 0]);
}

/// Two parts, the pelvis and a thigh 1 m below it sharing one position:
/// the merge places the thigh in the pelvis's frame, the weld keeps one of
/// the shared pair, and the `_` and `#` nodes stay out.
#[test]
fn the_body_parts_merge_and_weld_in_the_first_roots_frame() {
    let base = scene(vec![
        part(
            "01Pelvis",
            None,
            [0.0, 2.0, 0.0],
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        ),
        part(
            "02RULeg",
            Some(1),
            [0.0, -1.0, 0.0],
            &[[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        ),
        part(
            "_hidden",
            Some(1),
            [0.0; 3],
            &[[5.0; 3], [6.0; 3], [7.0; 3]],
        ),
        part(
            "#marker",
            Some(1),
            [0.0; 3],
            &[[5.0; 3], [6.0; 3], [7.0; 3]],
        ),
    ]);
    let (positions, records) = merged_body(&base).unwrap();
    assert_eq!(positions.len(), 5, "{positions:?}");
    assert_eq!(records.len(), 5);
    // The pelvis's two and the thigh's three left after the weld, the
    // thigh's in the pelvis's frame (1 m down): the weld's quicksort puts
    // the thigh's copy of the shared position first among the equal keys,
    // and the pelvis's merges into it.
    let at = |p: [f32; 3]| positions.iter().position(|&q| q == p).unwrap();
    for (p, bone) in [
        ([0.0, 0.0, 0.0], 2),
        ([1.0, 0.0, 0.0], 1),
        ([0.0, 0.0, 1.0], 1),
        ([1.0, -1.0, 0.0], 2),
        ([0.0, -1.0, 1.0], 2),
    ] {
        assert_eq!(records[at(p)].bones, [bone, 0, 0, 0], "{p:?}");
    }
    let near = body_skin_records(&base, &[[0.0, 0.01, 0.0], [0.9, -1.0, 0.1]]).unwrap();
    assert_eq!(near[0].bones[0], 2);
    assert_eq!(near[1].bones[0], 2);
}

/// What the shipped races' skin records and the garments that take them
/// look like (skipped without the game): the merged body's welded count,
/// how far each base position is from its record's position, and over
/// every garment piece asking for the body's skin, how the transferred
/// weights compare with the seam weights Exarust built before.
#[test]
fn shipped_bodies_hand_their_skin_to_the_garments() {
    let Some(dir) = crate::locate_install(None) else {
        return;
    };
    let game = crate::GameData::open(dir).unwrap();
    let resource = game.pack(PackKind::Resource).unwrap();
    let characters = game.pack(PackKind::Characters).unwrap();
    let parse = |name: &str| Scene::parse(resource.get(name).unwrap()).unwrap();
    for stem in ["human", "ancient"] {
        let morph_scene = parse(&format!("{stem}morph.rfc"));
        let base = parse(&format!("{stem}base.rfc"));
        let body = morph::MorphBody::of_scene(&morph_scene).unwrap();
        let started = std::time::Instant::now();
        let (positions, records) = merged_body(&base).unwrap();
        let skin = morph::nearest_skin_records(&body.set.base, &positions, &records);
        let took = started.elapsed();
        let root = morph::model_frames(&base)[0];
        // Distances from each base position to the merged position whose
        // record it takes (cm).
        let mut dist: Vec<f32> = body
            .set
            .base
            .iter()
            .map(|&p| {
                positions
                    .iter()
                    .map(|&q| {
                        let d = [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
                        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() * 100.0
                    })
                    .fold(f32::INFINITY, f32::min)
            })
            .collect();
        dist.sort_by(f32::total_cmp);
        let unweighted = skin.iter().filter(|r| r.bones[0] < 1).count();
        eprintln!(
            "{stem}: merged {} positions from the parts, {} base positions, records in {took:?}; \
             morph frame {:?}, first root frame {:?}; base to record position (cm): median {:.3}, \
             max {:.3}, over 1 cm {}; records without a joint {unweighted}",
            positions.len(),
            body.set.base.len(),
            body.frame,
            root,
            dist[dist.len() / 2],
            dist[dist.len() - 1],
            dist.iter().filter(|&&d| d > 1.0).count(),
        );
        assert_eq!(unweighted, 0);
        for r in &skin {
            let used = (0..4).take_while(|&k| r.bones[k] > 0);
            let sum: f32 = used.map(|k| r.weights[k]).sum();
            assert!((sum - 1.0).abs() < 1e-4, "{r:?}");
        }
        let body = body.with_skin(skin).unwrap();
        // The race's base parts' frames by bone, as the transfer places a
        // garment piece.
        let base_frames = morph::model_frames(&base);
        let bone_frame = |bone: usize| {
            base.nodes
                .iter()
                .zip(&base_frames)
                .find(|(n, _)| exa_formats::rsq::bone_index(&n.name) == Some(bone))
                .map(|(_, f)| *f)
        };
        let (mut pieces, mut flagged, mut unnamed, mut skipped) = (0, 0, 0, 0);
        let (mut vertices, mut moved_bone, mut unreached) = (0usize, 0usize, 0usize);
        let (mut own_seam, mut own_transfer) = (0usize, 0usize);
        let mut l1 = 0.0f64;
        for entry in characters.entries() {
            let Ok(scene) = Scene::parse(characters.bytes(entry)) else {
                continue;
            };
            let seam = scene_skins(&scene, true, PackKind::Characters, &[]);
            for (i, node) in scene.nodes.iter().enumerate() {
                let NodeBody::Mesh(mesh) = &node.body else {
                    continue;
                };
                pieces += 1;
                if mesh.render_flags & morph::BODY_SKIN == 0 {
                    continue;
                }
                flagged += 1;
                if node.flags & 0x20 != 0 {
                    skipped += 1;
                }
                let Some(frame) = exa_formats::rsq::bone_index(&node.name)
                    .filter(|b| (1..=18).contains(b))
                    .and_then(bone_frame)
                else {
                    unnamed += 1;
                    continue;
                };
                let m = body.transfer(mesh, &frame);
                let records = m.skin.unwrap();
                for (k, r) in records.iter().enumerate() {
                    assert!(
                        r.weights.iter().all(|w| w.is_finite()),
                        "{stem} {} {} element {k}: {r:?}",
                        entry.name,
                        node.name
                    );
                }
                unreached += records.iter().filter(|r| r.bones[0] < 1).count();
                let Some((own, seam)) = &seam[i] else {
                    continue;
                };
                for (v, &e) in mesh.position_ids.iter().enumerate() {
                    let r = vertex_of(&records[e as usize]);
                    let s = seam[v];
                    vertices += 1;
                    let top = |w: &VertexSkin| {
                        w.iter()
                            .copied()
                            .fold((0u16, f32::MIN), |a, b| if b.1 > a.1 { b } else { a })
                            .0
                    };
                    moved_bone += usize::from(top(&r) != top(&s));
                    own_seam += usize::from(top(&s) == *own);
                    own_transfer += usize::from(top(&r) == *own);
                    let mut by_bone = std::collections::HashMap::<u16, f32>::new();
                    for (b, w) in r {
                        *by_bone.entry(b).or_default() += w;
                    }
                    for (b, w) in s {
                        *by_bone.entry(b).or_default() -= w;
                    }
                    l1 += by_bone.values().map(|d| f64::from(d.abs())).sum::<f64>() * 0.5;
                }
            }
        }
        eprintln!(
            "{stem}: garment pieces {pieces}, asking for the body's skin {flagged} (skin flag 0x20: \
             {skipped}, not named after a body part: {unnamed}); over their {vertices} vertices: \
             dominant bone changed on {moved_bone} ({:.1}%), mean weight moved {:.3}; elements \
             never reached {unreached}; the piece's own bone leads on {:.1}% of the seam \
             weights and {:.1}% of the transferred ones",
            100.0 * moved_bone as f64 / vertices.max(1) as f64,
            l1 / vertices.max(1) as f64,
            100.0 * own_seam as f64 / vertices.max(1) as f64,
            100.0 * own_transfer as f64 / vertices.max(1) as f64,
        );
        assert!(flagged > 1000, "{flagged}");
    }
}
