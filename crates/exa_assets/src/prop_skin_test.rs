use super::*;
use bevy::math::Vec3;
use exa_formats::rfc::{Mesh, Node};

fn mesh_node(name: &str, parent: Option<usize>, render: u32, at: [f32; 3]) -> Node {
    Node {
        kind: 0x3D03,
        name: name.into(),
        flags: 0x801_0089,
        flags2: 0x2c_1010,
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
            render_flags: render,
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

/// The mattress's tree: the root, then its two children take bones 2 and
/// 3 before the walk enters the first, whose child takes 4; the second's
/// child takes 5. These are the bone numbers its stored streams use.
#[test]
fn the_walk_numbers_children_before_entering_them() {
    let s = scene(vec![
        mesh_node("A", None, 0xd0_0000, [0.0; 3]),
        mesh_node("D", Some(1), 0x50_0000, [0.0, 0.0, 0.2]),
        mesh_node("E", Some(2), 0x50_0000, [0.0, 0.0, 0.4]),
        mesh_node("B", Some(1), 0x50_0000, [0.0, 0.0, -0.2]),
        mesh_node("C", Some(4), 0x50_0000, [0.0, 0.0, -0.4]),
    ]);
    // The root's built bit (0x800000) does not stop it: only children are
    // tested for it.
    let plans = plan(&s);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].joints, [0, 0, 1, 3, 2, 4]);
    assert_eq!(plans[0].bone_of(2), Some(4));
    assert_eq!(plans[0].bone_of(4), Some(5));
}

/// A child without the skinned bit, or with the built bit, is not walked;
/// a root without the bit, an instance or an already skinned root is not
/// built.
#[test]
fn only_marked_meshes_join() {
    let mut s = scene(vec![
        mesh_node("root", None, 0x40_0000, [0.0; 3]),
        mesh_node("plain", Some(1), 0, [0.0; 3]),
        mesh_node("built", Some(1), 0xc0_0000, [0.0; 3]),
        mesh_node("under plain", Some(2), 0x40_0000, [0.0; 3]),
        mesh_node("other root", None, 0, [0.0; 3]),
    ]);
    let plans = plan(&s);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].members, [(0, 1)]);
    s.nodes[0].flags2 |= ALREADY_SKINNED;
    assert!(plan(&s).is_empty());
    s.nodes[0].flags2 = 0;
    s.nodes[0].instance = Some(5);
    assert!(plan(&s).is_empty());
}

/// At rest every bone moves a vertex of a member's mesh to where its own
/// node puts it.
#[test]
fn the_binds_agree_at_rest() {
    let s = scene(vec![
        mesh_node("root", None, 0x40_0000, [1.0, 2.0, 3.0]),
        mesh_node("a", Some(1), 0x40_0000, [0.0, -0.3, 0.0]),
        mesh_node("b", Some(2), 0x40_0000, [0.1, -0.3, 0.0]),
    ]);
    let plan = &plan(&s)[0];
    let binds = inverse_binds(&s, plan, 2);
    let v = Vec3::new(0.05, 0.1, -0.2);
    let expected = rest_frame(&s, 2).transform_point3(v);
    for (b, bind) in binds.iter().enumerate().skip(1) {
        let world = rest_frame(&s, plan.joints[b]) * *bind;
        assert!(world.transform_point3(v).abs_diff_eq(expected, 1e-6), "{b}");
    }
}

/// The shipped object libraries (with `EXANIMA_DIR`) hold 22 skinned
/// objects (one root each, Objlib 20, Objects 2) with 105 member meshes;
/// every member ships a skin stream whose bones are the walk's bone
/// numbers, its own among them, and the loader gives each member its
/// skinned parts.
#[test]
fn shipped_skinned_objects() {
    use crate::{ModelCache, ModelLoader, PackKind};
    use bevy::prelude::Assets;
    use exa_render::material::ExaMaterial;
    let Some(dir) = crate::locate_install(None) else {
        return;
    };
    let game = crate::GameData::open(dir).unwrap();
    let mut objects = Vec::new();
    let mut members = 0;
    for kind in [PackKind::Objlib, PackKind::Objects] {
        let pack = game.pack(kind).unwrap();
        for entry in pack.entries() {
            let Ok(scene) = Scene::parse(pack.bytes(entry)) else {
                continue;
            };
            for plan in plan(&scene) {
                objects.push((kind, entry.name.clone(), plan.members.len()));
                members += plan.members.len();
                for &(node, bone) in &plan.members {
                    let NodeBody::Mesh(mesh) = &scene.nodes[node].body else {
                        panic!("{}: member {node} is not a mesh", entry.name);
                    };
                    let skin = mesh.skin.as_ref().expect("a stored skin stream");
                    assert!(skin.len() >= mesh.positions.len());
                    let mut own = false;
                    for w in skin {
                        let sum: f32 = w.weights.iter().sum();
                        assert!((sum - 1.0).abs() < 1e-3, "{}: {sum}", entry.name);
                        for (&b, &weight) in w.bones.iter().zip(&w.weights) {
                            if weight > 0.0 {
                                assert!(
                                    (1..plan.joints.len()).contains(&(b as usize)),
                                    "{}: bone {b}",
                                    entry.name
                                );
                                own |= b == u32::from(bone);
                            }
                        }
                    }
                    assert!(own, "{}: node {node} never on its own bone", entry.name);
                }
            }
        }
    }
    eprintln!("{objects:?}");
    assert_eq!(objects.len(), 22);
    assert_eq!(
        objects.iter().filter(|o| o.0 == PackKind::Objects).count(),
        2
    );
    assert_eq!(members, 105);

    let mut cache = ModelCache::default();
    let mut meshes = Assets::<bevy::prelude::Mesh>::default();
    let mut materials = Assets::<ExaMaterial>::default();
    let mut images = Assets::<bevy::prelude::Image>::default();
    let mut loader = ModelLoader {
        game: &game,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: false,
    };
    for (kind, entry, count) in &objects {
        let entry = entry.trim_end_matches(".rfc");
        let model = loader.model(*kind, entry).unwrap();
        let skinned: Vec<_> = model
            .nodes
            .iter()
            .filter_map(|n| n.prop_skin.as_ref().map(|s| (n, s)))
            .collect();
        assert_eq!(skinned.len(), *count, "{entry}");
        for (node, skin) in skinned {
            assert_eq!(skin.parts.len(), node.parts.len(), "{entry}");
            assert_eq!(skin.joints.len(), skin.inverse_binds.len());
            assert!(node.skin_bone.is_none());
        }
    }
}
