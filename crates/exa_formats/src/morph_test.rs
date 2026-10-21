//! Tests of the morph sets, the physique weights and the transfer.
//! (Included from morph.rs: `#[path = "morph_test.rs"] mod tests;`)

use super::*;

fn bytes(i: &[i32], f: &[f32]) -> Vec<u8> {
    let mut b: Vec<u8> = i.iter().flat_map(|x| x.to_le_bytes()).collect();
    b.extend(f.iter().flat_map(|x| x.to_le_bytes()));
    b
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

fn close3(a: Vec3, b: Vec3) -> bool {
    (0..3).all(|k| close(a[k], b[k]))
}

/// Two landmarks, three positions, then the target count and two targets.
fn payload() -> Vec<u8> {
    let mut b = bytes(&[2], &[0.0, 0.0, 0.0, -9.07, -8.46, 3.5]);
    b.extend(bytes(&[], &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]));
    b.extend(bytes(&[2], &[]));
    for t in 0..2 {
        let v = t as f32 + 1.0;
        b.extend(bytes(&[], &[v; 6]));
        b.extend(bytes(&[], &[-v; 9]));
    }
    b
}

#[test]
fn a_set_decodes_in_metres() {
    let set = MorphSet::parse(&payload(), 3).unwrap();
    assert_eq!(set.landmarks.len(), 2);
    assert!(close3(set.landmarks[1], [-0.0907, -0.0846, 0.035]));
    assert!(close3(set.base[2], [0.07, 0.08, 0.09]));
    assert_eq!(set.targets.len(), 2);
    assert!(close3(set.targets[1].landmarks[0], [0.02; 3]));
    assert!(close3(set.targets[0].positions[2], [-0.01; 3]));
    // Trailing bytes are left alone.
    let mut longer = payload();
    longer.extend([0u8; 7]);
    assert_eq!(MorphSet::parse(&longer, 3).unwrap(), set);
    // At weights 1 and ½: base − 1 cm − 1 cm.
    let p = set.positions(&[1.0, 0.5]);
    assert!(close3(p[0], [-0.01, 0.0, 0.01]));
}

#[test]
fn malformed_sets_are_errors() {
    let b = payload();
    for cut in [0, 3, 10, b.len() - 1] {
        assert!(MorphSet::parse(&b[..cut], 3).is_err(), "cut at {cut}");
    }
    // More positions than the payload holds.
    assert!(MorphSet::parse(&b, 400).is_err());
    // Negative counts.
    assert!(MorphSet::parse(&bytes(&[-1], &[]), 0).is_err());
    let mut neg = bytes(&[0], &[]);
    neg.extend(bytes(&[-3], &[]));
    assert!(MorphSet::parse(&neg, 0).is_err());
    // A huge target count over a short payload.
    let mut huge = bytes(&[0], &[]);
    huge.extend(bytes(&[i32::MAX], &[]));
    assert!(MorphSet::parse(&huge, 1).is_err());
}

#[test]
fn the_smallest_weight_is_the_originals() {
    assert!((MIN_WEIGHT - 1.192e-7).abs() < 1e-10, "{MIN_WEIGHT}");
    assert!(!morphs(&[0.0, 1e-7, -1e-7]));
    assert!(morphs(&[0.0, 2e-7]));
}

#[test]
fn the_neutral_physique_weighs_nothing() {
    let w = Physique::NEUTRAL.weights(TARGETS);
    assert_eq!(w, vec![0.0; TARGETS]);
    assert!(!morphs(&w));
}

#[test]
fn a_new_record_takes_the_fourth_corner() {
    // Physique (0, 0): x stays 0 (only positive x is adjusted), y′ = 1.
    let w = Physique::of_record(&CharacterRecord::default()).weights(TARGETS);
    let mut want = vec![0.0; TARGETS];
    want[3] = 1.0;
    assert_eq!(w, want);
}

#[test]
fn a_broad_old_feral_woman() {
    // x 0.8 → 0.74 → 0.691 (height ½) → 0.6285 (feral ½); y′ 0.7, u 0.257.
    let p = Physique {
        build: [0.8, 0.3],
        height: 0.5,
        age: 0.5,
        feral: 0.5,
        female: true,
    };
    let w = p.weights(TARGETS);
    let want = [
        0.0, 0.212025, 0.61005, 0.0, 0.526094, 1.0, 0.1875, 0.0, 0.0, 0.0, 0.5,
    ];
    for (i, (a, b)) in w.iter().zip(want).enumerate() {
        assert!((a - b).abs() < 1e-5, "w{i}: {a} vs {b}");
    }
}

#[test]
fn a_slight_tall_young_man() {
    // x 0.2 → 0.25 (height −0.4); y′ 0.4, u ½.
    let p = Physique {
        build: [0.2, 0.6],
        height: -0.4,
        age: 0.0,
        feral: 0.0,
        female: false,
    };
    let w = p.weights(TARGETS);
    let want = [0.3, 0.0, 0.2, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.4];
    for (i, (a, b)) in w.iter().zip(want).enumerate() {
        assert!((a - b).abs() < 1e-6, "w{i}: {a} vs {b}");
    }
}

#[test]
fn the_feral_level_pulls_past_the_corner() {
    // Physique (0, 1), feral 1: x = −¼, u = −½: w0 = 1.5, and w4 = ½.
    let p = Physique {
        build: [0.0, 1.0],
        height: 0.0,
        age: 0.0,
        feral: 1.0,
        female: false,
    };
    let w = p.weights(TARGETS);
    assert!(close(w[0], 1.5) && close(w[AGE], 0.5), "{w:?}");
    assert!(w[1..AGE].iter().all(|&v| v == 0.0), "{w:?}");
}

#[test]
fn physique_values_are_kept_in_range() {
    let wild = Physique {
        build: [f32::NAN, 7.0],
        ..Physique::NEUTRAL
    };
    // NaN reads as 0, 7 as 1: x 0, y′ 0 → all on the first corner.
    let mut want = vec![0.0; TARGETS];
    want[0] = 1.0;
    assert_eq!(wild.weights(TARGETS), want);
    // A record's height, age and feral level are clamped by the builder.
    let r = CharacterRecord {
        height: 3.0,
        age: -1.0,
        feral: 2.0,
        ..CharacterRecord::default()
    };
    let p = Physique::of_record(&r);
    assert_eq!((p.height, p.age, p.feral), (1.0, 0.0, 1.0));
}

#[test]
fn small_sets_take_only_their_targets() {
    let r = CharacterRecord::default();
    // The picker writes only into a set of more than three targets.
    assert_eq!(Physique::of_record(&r).weights(3), vec![0.0; 3]);
    assert_eq!(Physique::of_record(&r).weights(0), Vec::<f32>::new());
    let tall = Physique {
        height: -1.0,
        ..Physique::NEUTRAL
    };
    assert_eq!(tall.weights(TARGETS)[HEIGHT], -1.0);
    assert_eq!(tall.weights(HEIGHT).len(), HEIGHT);
}

const IDENTITY: Frame = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0; 3]];

/// A mesh whose vertices are its position elements.
fn mesh(positions: &[Vec3], triangles: &[[u32; 3]]) -> Mesh {
    Mesh {
        positions: positions.to_vec(),
        normals: vec![[0.0, 1.0, 0.0]; positions.len()],
        position_ids: (0..positions.len() as u32).collect(),
        position_count: positions.len(),
        smoothing: (0..positions.len() as u32).collect(),
        indices: triangles.iter().flatten().copied().collect(),
        ..Mesh::default()
    }
}

/// A flat square body, 1 m a side at y = 0 in its space, placed 1 m up:
/// target 0 lifts every position 2 cm and landmark 0 1 cm; target 1 moves
/// corner 0 10 cm along x.
fn square_body() -> MorphBody {
    let base = vec![
        [-0.5, 0.0, -0.5],
        [0.5, 0.0, -0.5],
        [0.5, 0.0, 0.5],
        [-0.5, 0.0, 0.5],
    ];
    let set = MorphSet {
        landmarks: vec![[0.0; 3], [0.0, 1.0, 0.0]],
        base: base.clone(),
        targets: vec![
            MorphTarget {
                landmarks: vec![[0.0, 0.01, 0.0], [0.0, 0.05, 0.0]],
                positions: vec![[0.0, 0.02, 0.0]; 4],
            },
            MorphTarget {
                landmarks: vec![[0.0; 3]; 2],
                positions: vec![[0.1, 0.0, 0.0], [0.0; 3], [0.0; 3], [0.0; 3]],
            },
        ],
    };
    let mut frame = IDENTITY;
    frame[3] = [0.0, 1.0, 0.0];
    MorphBody::new(set, &mesh(&base, &[[0, 1, 2], [0, 2, 3]]), frame).unwrap()
}

#[test]
fn landmarks_are_found_from_the_bodys_place() {
    let body = square_body();
    // The landmarks sit 1 m up in the model's space.
    assert_eq!(body.nearest_landmark([0.0, 1.1, 0.0]), Some(0));
    assert_eq!(body.nearest_landmark([0.0, 1.6, 0.0]), Some(1));
    // A set without landmarks has none to give.
    let mut empty = square_body();
    empty.set.landmarks.clear();
    assert_eq!(empty.nearest_landmark([0.0; 3]), None);
}

#[test]
fn a_body_must_match_its_mesh() {
    let body = square_body();
    let short = mesh(&body.set.base[..3], &[[0, 1, 2]]);
    assert!(MorphBody::new(body.set.clone(), &short, IDENTITY).is_none());
}

#[test]
fn parts_follow_the_nearest_surface() {
    let body = square_body();
    // In the part's space (placed 1.02 m up): elements 0 and 3 5 cm above
    // the body, element 1 30 cm above (beyond the reach) beside them, and a
    // triangle 50 cm up that no element near the body touches.
    let part = mesh(
        &[
            [0.2, 0.03, 0.1],
            [0.2, 0.28, 0.1],
            [0.0; 3],
            [0.25, 0.03, 0.1],
            [0.0, 0.48, 0.0],
            [0.1, 0.48, 0.0],
            [0.0, 0.48, 0.1],
        ],
        &[[0, 1, 3], [4, 5, 6]],
    );
    let mut frame = IDENTITY;
    frame[3] = [0.0, 1.02, 0.0];
    let m = body.transfer(&part, &frame);
    // The origin moves with landmark 0.
    assert!(close3(m.origin[0], [0.0, 0.01, 0.0]));
    assert!(close3(m.origin[1], [0.0; 3]));
    let lift = |t: usize, e: usize| m.positions[t][e];
    // On the body: its 2 cm less the landmark's 1 cm.
    assert!(close3(lift(0, 0), [0.0, 0.01, 0.0]));
    assert!(close3(lift(0, 3), [0.0, 0.01, 0.0]));
    // Away from it: its only neighbour (the next corner, element 3).
    assert!(close3(lift(0, 1), lift(0, 3)));
    // Element 2 sits at the part's origin, 2 cm above the body: matched.
    assert!(close3(lift(0, 2), [0.0, 0.01, 0.0]));
    // Never reached: it stays put while the origin rises.
    assert!(close3(lift(0, 4), [0.0, -0.01, 0.0]));
    // Target 1: element 0 lies over (0.2, 0.1), 0.3 of corner 0.
    assert!(close3(lift(1, 0), [0.03, 0.0, 0.0]), "{:?}", lift(1, 0));
    // At weights ½ and 1.
    let (origin, offsets) = m.offsets(&[0.5, 1.0]);
    assert!(close3(origin, [0.0, 0.005, 0.0]));
    assert!(close3(offsets[0], [0.03, 0.005, 0.0]));
    // Weights below the smallest leave everything.
    let (origin, offsets) = m.offsets(&[1e-8, -1e-8]);
    assert_eq!(origin, [0.0; 3]);
    assert!(offsets.iter().all(|&d| d == [0.0; 3]));
}

#[test]
fn offsets_are_taken_into_the_parts_frame() {
    let body = square_body();
    let part = mesh(
        &[[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.0, 0.1]],
        &[[0, 1, 2]],
    );
    // Turned a quarter about z: the part's x is the model's y.
    let frame = [
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.05, 0.0],
    ];
    let m = body.transfer(&part, &frame);
    // Element 0 lies 5 cm above the body: the 2 cm lift less the
    // landmark's 1 cm, along the model's y, is along the part's x.
    let d = m.positions[0][0];
    assert!(close3(d, [0.01, 0.0, 0.0]), "{d:?}");
}

#[test]
fn normals_follow_the_new_shape() {
    let mut m = mesh(&[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]], &[[0, 1, 2]]);
    m.recompute_normals();
    assert!(close3(m.normals[0], [0.0, 1.0, 0.0]));
    // Tipped up along x, the face turns toward −x.
    morph_mesh(&mut m, &[[0.0; 3], [0.0, 1.0, 0.0], [0.0; 3]]);
    let n = m.normals[0];
    assert!(close3(n, [-0.70710677, 0.70710677, 0.0]), "{n:?}");
    // A mesh with stray indices or keys never panics.
    let mut bad = mesh(&[[0.0; 3]], &[[0, 4, 9]]);
    bad.smoothing.clear();
    bad.position_ids.clear();
    bad.recompute_normals();
    assert_eq!(bad.normals, vec![[0.0, 1.0, 0.0]]);
}

fn node(name: &str, parent: Option<usize>, transform: Frame, body: NodeBody) -> Node {
    Node {
        kind: crate::rfc::NODE_MESH,
        name: name.into(),
        flags: 0,
        flags2: 0,
        parent,
        transform,
        bounds_centre: [0.0; 3],
        bounds_half_extents: [0.0; 3],
        bounds_radius: 0.0,
        rigid_body: None,
        cloth_marker: false,
        key_tracks: Vec::new(),
        morph: None,
        instance: None,
        unknown_sub_chunks: Vec::new(),
        body,
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
        local_strings: None,
        local_string_error: None,
        placed_objects: Vec::new(),
        unknown_chunks: Vec::new(),
        level_errors: Vec::new(),
    }
}

#[test]
fn bounds_keep_a_least_extent() {
    let (c, h, r) = bounds(&[[0.0, 1.0, 0.0], [0.2, 1.0, 0.0]]);
    assert!(close3(c, [0.1, 1.0, 0.0]) && close(r, 0.1));
    // A flat part keeps 0.001 cm.
    assert_eq!(h, [0.1, MIN_HALF_EXTENT, MIN_HALF_EXTENT]);
    let (_, h, _) = bounds(&[]);
    assert_eq!(h, [MIN_HALF_EXTENT; 3]);
}

#[test]
fn frames_compose_down_the_tree() {
    let turned = [
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
    ];
    let mut child = IDENTITY;
    child[3] = [0.1, 0.0, 0.0];
    let s = scene(vec![
        node("root", None, turned, NodeBody::NotMesh),
        node("child", Some(1), child, NodeBody::NotMesh),
        // Its own parent: a loop is cut.
        node("loop", Some(3), IDENTITY, NodeBody::NotMesh),
    ]);
    let f = model_frames(&s);
    // The child's 10 cm along the root's x is 10 cm up.
    assert!(close3(f[1][3], [0.0, 1.1, 0.0]));
    assert!(close3(f[1][0], [0.0, 1.0, 0.0]));
    assert_eq!(f[2], IDENTITY);
    assert!(close3(unplace(&f[1], [0.0, 1.1, 0.0]), [0.0; 3]));
}

#[test]
fn morphed_origins_move_the_joints() {
    let quad = |y: f32| {
        NodeBody::Mesh(Box::new(mesh(
            &[[0.0, y, 0.0], [0.1, y, 0.0], [0.0, y, -0.1]],
            &[[0, 1, 2]],
        )))
    };
    let turned = [
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
    ];
    let mut local = IDENTITY;
    local[3] = [0.2, 0.0, 0.0];
    let mut s = scene(vec![
        node("pelvis", None, turned, quad(0.0)),
        node("thigh", Some(1), local, quad(0.0)),
        node("plain", Some(1), local, NodeBody::NotMesh),
        node("under plain", Some(3), local, quad(0.0)),
    ]);
    let part = |origin: Vec3| PartMorph {
        origin: vec![origin],
        positions: vec![vec![[0.0, 0.05, 0.0], [0.0; 3], [0.0; 3]]],
    };
    let parts = vec![
        Some(part([0.0, 0.1, 0.0])),
        Some(part([0.03, 0.0, 0.0])),
        None,
        Some(part([0.0, 0.5, 0.0])),
    ];
    morph_scene(&mut s, &parts, &[1.0]);
    // The root's origin rises 10 cm.
    assert!(close3(s.nodes[0].transform[3], [0.0, 1.1, 0.0]));
    // The thigh: model origin (0, 1.2, 0) + (0.03, 0, 0) against the
    // pelvis at (0, 1.1, 0): 0.1 m along the pelvis's x (the model's y),
    // 0.03 m along its −y.
    let t = s.nodes[1].transform[3];
    assert!(close3(t, [0.1, -0.03, 0.0]), "{t:?}");
    // A node that is not morphed ends the walk: the node under it keeps
    // its translation though its mesh is morphed.
    assert_eq!(s.nodes[3].transform[3], local[3]);
    let NodeBody::Mesh(m) = &s.nodes[3].body else {
        panic!()
    };
    assert!(close(m.positions[0][1], 0.05));
    // The moved vertex turned the normals and stretched the box.
    let NodeBody::Mesh(m) = &s.nodes[0].body else {
        panic!()
    };
    assert!(m.normals[0][0] > 0.0, "{:?}", m.normals[0]);
    let n = &s.nodes[0];
    assert!(close3(n.bounds_centre, [0.05, 0.025, -0.05]));
    assert!(close3(n.bounds_half_extents, [0.05, 0.025, 0.05]));
    assert!(close(
        n.bounds_radius,
        (0.05f32 * 0.05 * 2.0 + 0.025 * 0.025).sqrt()
    ));
}

/// The installed game's packs, when `EXANIMA_DIR` (or `.env`) names them.
fn game() -> Option<exa_rpk::GameData> {
    exa_rpk::GameData::open(exa_rpk::locate_install(None)?).ok()
}

/// Every `*morph.rfc` in `Resource.rpk`: its first morphed node, its mesh
/// and its frame.
fn shipped_sets(game: &exa_rpk::GameData) -> Vec<(String, Scene)> {
    let pack = game.pack(exa_rpk::PackKind::Resource).unwrap();
    pack.entries()
        .iter()
        .filter(|e| e.name.to_ascii_lowercase().ends_with("morph.rfc"))
        .map(|e| {
            (
                e.name.clone(),
                Scene::parse(pack.bytes(e)).expect("morph file decodes"),
            )
        })
        .collect()
}

#[test]
fn shipped_morph_sets() {
    let Some(game) = game() else {
        return;
    };
    let sets = shipped_sets(&game);
    let names: Vec<&str> = sets.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names.len(), 2, "{names:?}");
    for (name, scene) in &sets {
        let carriers: Vec<&Node> = scene.nodes.iter().filter(|n| n.morph.is_some()).collect();
        assert_eq!(carriers.len(), 1, "{name}");
        let node = carriers[0];
        let set = node.morph.as_ref().unwrap().as_ref().expect("decodes");
        let NodeBody::Mesh(mesh) = &node.body else {
            panic!("{name}: not a mesh")
        };
        assert_eq!(
            (set.landmarks.len(), set.base.len(), set.targets.len()),
            (18, 2286, TARGETS),
            "{name}"
        );
        assert_eq!(mesh.position_count, set.base.len(), "{name}");
        // Landmark 1 is the right hip, (−9.07, −8.46, 3.50) cm.
        let hip = set.landmarks[1].map(|v| v * 100.0);
        let want = [-9.07, -8.46, 3.50];
        assert!(
            (0..3).all(|k| (hip[k] - want[k]).abs() < 0.005),
            "{name}: {hip:?}"
        );
        // The height target brings the body down onto its feet: the pelvis
        // 11.2 cm (the standing pelvis drops 11.25 cm per unit of height),
        // the knees 6.2, the head 13.6, the feet (landmarks 3 and 6) not at
        // all, every other joint 10–15 cm.
        let height = &set.targets[HEIGHT];
        let drops: Vec<f32> = height.landmarks.iter().map(|d| d[1] * 100.0).collect();
        assert!((drops[0] + 11.20).abs() < 0.01, "{drops:?}");
        assert!((drops[2] + 6.21).abs() < 0.01 && (drops[9] + 13.59).abs() < 0.01);
        for (i, d) in drops.iter().enumerate() {
            match i {
                3 | 6 => assert!(d.abs() < 0.001, "{i}: {d}"),
                2 | 5 => assert!((d + 6.21).abs() < 0.01, "{i}: {d}"),
                _ => assert!((-15.0..-10.0).contains(d), "{i}: {d}"),
            }
        }
        let body = MorphBody::of_scene(scene).expect("a morph body");
        assert_eq!(body.set.base.len(), 2286);
    }
}

#[test]
fn shipped_parts_take_the_morphs() {
    let Some(game) = game() else {
        return;
    };
    let sets = shipped_sets(&game);
    let (_, morph) = sets
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("humanmorph.rfc"))
        .unwrap();
    let body = MorphBody::of_scene(morph).unwrap();
    let pack = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let base = Scene::parse(pack.get("humanbase.rfc").unwrap()).unwrap();
    let frames = model_frames(&base);
    let mut parts = 0;
    for (i, node) in base.nodes.iter().enumerate() {
        let NodeBody::Mesh(mesh) = &node.body else {
            continue;
        };
        parts += 1;
        let bone = crate::rsq::bone_index(&node.name).expect("a body part");
        // Each part's origin is its own landmark.
        assert_eq!(
            body.nearest_landmark(frames[i][3]),
            Some(bone - 1),
            "{}",
            node.name
        );
        // Every element of every part has the body's surface within reach.
        let elements = element_positions(mesh);
        assert!(elements.iter().all(|&p| {
            let q = unplace(&body.frame, place(&frames[i], p));
            body.nearest_surface(q).is_some()
        }));
        let m = body.transfer(mesh, &frames[i]);
        assert_eq!(m.positions.len(), TARGETS);
        assert!(
            m.positions
                .iter()
                .flatten()
                .flatten()
                .all(|v| v.is_finite())
        );
        // The height target takes the parts down with their landmarks: the
        // pelvis 11.20 cm, the right thigh 11.92 cm and 1.50 cm inward.
        let o = m.origin[HEIGHT].map(|v| v * 100.0);
        match node.name.as_str() {
            "01Pelvis" => assert!((o[1] + 11.196).abs() < 0.01, "{o:?}"),
            "02RULeg" => assert!((o[0] - 1.499).abs() < 0.01 && (o[1] + 11.920).abs() < 0.01),
            "04RFoot" => assert!(o[1].abs() < 0.01, "{o:?}"),
            _ => {}
        }
    }
    assert_eq!(parts, 18);
    // The body stands where the base body's pelvis does, unturned.
    assert!((frames[0][3][1] * 100.0 - 102.98).abs() < 0.01);
    assert!(close3(body.frame[3], frames[0][3]) && close3(body.frame[0], [1.0, 0.0, 0.0]));
}

#[test]
fn far_flung_sets_never_panic() {
    // Targets of no landmarks and no positions hold nothing: a huge count
    // of them is an error at once, not two billion empty targets.
    let mut nothing = bytes(&[0], &[]);
    nothing.extend(bytes(&[i32::MAX], &[]));
    assert!(MorphSet::parse(&nothing, 0).is_err());
    let mut one = bytes(&[0], &[]);
    one.extend(bytes(&[1], &[]));
    assert!(MorphSet::parse(&one, 0).is_err());
    // A body spanning the float range builds a grid of at most 64 cells a
    // side; a part near it finds no surface and stays put.
    let base = vec![[-3e38, 0.0, 0.0], [3e38, 0.0, 0.0], [0.0, 3e38, 0.0]];
    let set = MorphSet {
        landmarks: vec![[0.0; 3]],
        base: base.clone(),
        targets: vec![MorphTarget {
            landmarks: vec![[0.0; 3]],
            positions: vec![[0.0; 3]; 3],
        }],
    };
    let body = MorphBody::new(set, &mesh(&base, &[[0, 1, 2]]), IDENTITY).unwrap();
    assert_eq!(body.grid.dims, [64, 64, 1]);
    let part = mesh(&[[0.0; 3], [0.1, 0.0, 0.0], [0.0, 0.1, 0.0]], &[[0, 1, 2]]);
    let m = body.transfer(&part, &IDENTITY);
    assert_eq!(m.positions[0], vec![[0.0; 3]; 3]);
}
