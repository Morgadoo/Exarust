//! The save's part records and mechanism words over a shipped level, with
//! the live world and physics plugins (skipped without the game).

use super::*;
use bevy::ecs::world::CommandQueue;
use bevy::time::TimeUpdateStrategy;
use exa_formats::level::PropAttachment;
use exa_physics::body::{DoorBody, LeverBody, Ragdoll};
use exa_world::saved_parts::{PartKind, PartTree};

#[derive(Resource, Default)]
struct Out(Option<(String, Vec<u8>)>);

/// `exanima01` loaded (from `saved` when given) into a headless app with the
/// physics and mechanism plugins and a save system that writes the section
/// on request.
fn level(game: &exa_assets::GameData, saved: Option<std::sync::Arc<[u8]>>) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::transform::TransformPlugin,
        exa_physics::PhysicsPlugin,
        exa_world::mechanisms::MechanismPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / exa_physics::STEP_HZ),
    ))
    .init_resource::<Out>();
    let mut cache = exa_assets::ModelCache::default();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
    let mut images = Assets::<Image>::default();
    let mut queue = CommandQueue::default();
    {
        let mut loader = exa_assets::ModelLoader {
            game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let mut commands = Commands::new(&mut queue, app.world());
        let mut load =
            exa_world::LevelLoad::begin_from(&mut commands, &mut loader, "exanima01", saved)
                .expect("exanima01");
        while !matches!(
            load.step(&mut commands, &mut loader, std::time::Duration::MAX),
            exa_world::LoadProgress::Finished(..)
        ) {}
    }
    queue.apply(app.world_mut());
    app.world_mut().insert_resource(meshes);
    for _ in 0..4 {
        app.update();
    }
    app
}

/// The placed object an entity belongs to, with its index in the section.
fn owner(app: &App, entity: Entity) -> Option<(Entity, u32)> {
    let mut at = entity;
    loop {
        if let Some(index) = app.world().get::<exa_world::PlacedIndex>(at) {
            return Some((at, index.0));
        }
        at = app.world().get::<ChildOf>(at)?.parent();
    }
}

/// Where a placed object stands (m).
fn placed_at(app: &App, object: Entity) -> Vec3 {
    app.world()
        .get::<GlobalTransform>(object)
        .unwrap()
        .translation()
}

/// The leaf's model node: its name and its tree path from the object's
/// node (the first root).
fn leaf_name(app: &App, object: Entity, leaf: Entity) -> String {
    let tree = app.world().get::<PartTree>(object).expect("a part tree");
    let node = tree
        .nodes
        .iter()
        .find(|n| n.entity == leaf)
        .expect("the leaf node");
    assert_eq!(node.kind, PartKind::Body);
    node.name.clone()
}

fn find_record<'a>(list: &'a [PropAttachment], name: &str) -> Option<&'a [[f32; 3]; 4]> {
    list.iter().find_map(|r| match r {
        PropAttachment::Transform {
            name: n, transform, ..
        } if n == name => Some(transform),
        PropAttachment::Transform { children, .. } => find_record(children, name),
        _ => None,
    })
}

/// A door leaf pulled open writes its live frame as its object's part
/// record and the door's stored word as the placement word; a lever's armed
/// word is written whole. A load of the written section spawns the leaf
/// where it was saved and the door and lever take their saved words; the
/// objects nothing touched keep their records and words.
#[test]
fn an_opened_door_and_an_armed_lever_come_back_from_the_save() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let Ok(game) = exa_assets::GameData::open(&dir) else {
        return;
    };
    let mut app = level(&game, None);
    let source = Scene::parse(&app.world().resource::<exa_world::LevelSource>().bytes).unwrap();
    // A shut, unlatched hinged leaf of a placed door.
    let mut q = app.world_mut().query::<(Entity, &DoorBody)>();
    let leaves: Vec<Entity> = q
        .iter(app.world())
        .filter(|(_, g)| g.door.closed && !g.latched() && !g.door.has_partner)
        .map(|(e, _)| e)
        .collect();
    let leaf = *leaves
        .iter()
        .find(|&&e| owner(&app, e).is_some())
        .expect("a shut unlatched single door");
    let (object, index) = owner(&app, leaf).unwrap();
    let door_at = placed_at(&app, object);
    let name = leaf_name(&app, object, leaf);
    let start = app.world().get::<DoorBody>(leaf).unwrap().door.angle;
    // Pull it open as a hand does.
    let anchor = |app: &App, share: f32| {
        let body = app.world().get::<Ragdoll>(leaf).unwrap();
        let glue = app.world().get::<DoorBody>(leaf).unwrap();
        let shape = glue.door.shape;
        let origin = glue.origin(body);
        let toward = glue.stop_at(body, shape.closed() - 2.0 * shape.half_swing * share) - origin;
        origin + Vec3::Y + Vec3::new(toward.x, 0.0, toward.z).normalize()
    };
    let grip = anchor(&app, 0.0);
    {
        let mut entity = app.world_mut().entity_mut(leaf);
        let mut glue = entity.take::<DoorBody>().unwrap();
        let mut body = entity.get_mut::<Ragdoll>().unwrap();
        glue.take_hold(&mut body, grip);
        entity.insert(glue);
    }
    for _ in 0..400 {
        let to = anchor(&app, 0.9);
        let mut entity = app.world_mut().entity_mut(leaf);
        let mut glue = entity.take::<DoorBody>().unwrap();
        let mut body = entity.get_mut::<Ragdoll>().unwrap();
        glue.pull_to(&mut body, Some(to));
        entity.insert(glue);
        app.update();
    }
    {
        let mut entity = app.world_mut().entity_mut(leaf);
        let mut glue = entity.take::<DoorBody>().unwrap();
        let mut body = entity.get_mut::<Ragdoll>().unwrap();
        glue.pull_to(&mut body, None);
        entity.insert(glue);
    }
    let glue = app.world().get::<DoorBody>(leaf).unwrap();
    assert!((glue.door.angle - start).abs() > 0.5, "the leaf swung open");
    let door_word = glue.door.stored() as u32;
    let open = app.world().get::<Ragdoll>(leaf).unwrap().parts[0].transform();
    // A lever with its return armed: the remaining time in the word's high
    // half.
    let mut q = app.world_mut().query::<(Entity, &LeverBody)>();
    let lever = q
        .iter(app.world())
        .map(|(e, _)| e)
        .find(|&e| owner(&app, e).is_some())
        .expect("a placed lever");
    let (lever_object, lever_index) = owner(&app, lever).unwrap();
    let lever_at = placed_at(&app, lever_object);
    let lever_word = {
        let mut glue = app.world_mut().get_mut::<LeverBody>(lever).unwrap();
        glue.lever.word = (glue.lever.word & 0xFFFF) | (37 << 16);
        glue.lever.word as u32
    };
    // The live database's local tier is the level's own.
    let mut db = exa_formats::objdb::ObjectDb::default();
    *db.store_mut(exa_formats::objdb::Tier::Local) = source.local_database.clone().unwrap();
    *db.library_mut(exa_formats::objdb::Tier::Local) =
        source.local_strings.clone().unwrap_or_default();
    app.add_systems(
        Update,
        move |mut scene: LevelSaveScene, mut out: ResMut<Out>| {
            if out.0.is_none() {
                out.0 = scene.section(&db, false, None);
            }
        },
    );
    app.update();
    let (locale, bytes) = app.world().resource::<Out>().0.clone().expect("a section");
    assert_eq!(locale, "exanima01");
    let back = Scene::parse(&bytes).unwrap();
    assert_eq!(back.placed_objects.len(), source.placed_objects.len());
    let door = &back.placed_objects[index as usize];
    assert_eq!(door.flags, door_word, "the door's word");
    assert_eq!(back.placed_objects[lever_index as usize].flags, lever_word);
    let record = find_record(&door.attachments, &name).expect("the leaf's part record");
    let saved = Affine3A::from_cols_array_2d(&[record[0], record[1], record[2], record[3]]);
    assert!(
        saved.abs_diff_eq(open, 1e-3),
        "record {saved:?} live {open:?}"
    );
    // The other objects whose records changed are bodies that moved in the
    // ticks (this app has no level to rest on); each is printed.
    let rewritten: Vec<usize> = source
        .placed_objects
        .iter()
        .zip(&back.placed_objects)
        .enumerate()
        .filter(|(_, (a, b))| a.attachments != b.attachments)
        .map(|(i, _)| i)
        .collect();
    eprintln!(
        "{} of {} placed objects rewrote their part records: {rewritten:?}",
        rewritten.len(),
        source.placed_objects.len()
    );
    for &i in rewritten.iter().take(6) {
        let flat = |list: &[PropAttachment]| -> Vec<[f32; 3]> {
            fn walk(list: &[PropAttachment], out: &mut Vec<[f32; 3]>) {
                for r in list {
                    if let PropAttachment::Transform {
                        transform,
                        children,
                        ..
                    } = r
                    {
                        out.push(transform[3]);
                        walk(children, out);
                    }
                }
            }
            let mut out = Vec::new();
            walk(list, &mut out);
            out
        };
        eprintln!(
            "object {i}: records {:?} -> {:?}",
            flat(&source.placed_objects[i].attachments),
            flat(&back.placed_objects[i].attachments)
        );
    }
    assert!(rewritten.contains(&(index as usize)));

    // The written section loads with the leaf where it was saved and the
    // saved words on the door and the lever.
    let mut again = level(&game, Some(bytes.into()));
    let mut q = again.world_mut().query::<(Entity, &DoorBody)>();
    let leaves: Vec<(Entity, DoorBody)> =
        q.iter(again.world()).map(|(e, g)| (e, g.clone())).collect();
    let (leaf, glue) = leaves
        .iter()
        .find(|(e, _)| {
            owner(&again, *e).is_some_and(|(o, _)| placed_at(&again, o).distance(door_at) < 0.01)
        })
        .expect("the door's leaf");
    assert_eq!(glue.door.stored() as u32, door_word);
    let loaded = again.world().get::<Ragdoll>(*leaf).unwrap().parts[0].transform();
    assert!(
        loaded.translation.distance(open.translation) < 0.01
            && loaded.matrix3.abs_diff_eq(open.matrix3, 0.01),
        "loaded {loaded:?} saved {open:?}"
    );
    let mut q = again.world_mut().query::<(Entity, &LeverBody)>();
    let words: Vec<(Entity, i32)> = q
        .iter(again.world())
        .map(|(e, g)| (e, g.lever.word))
        .collect();
    let nearest = words
        .iter()
        .filter_map(|(e, _)| owner(&again, *e))
        .map(|(o, i)| (i, placed_at(&again, o).distance(lever_at)))
        .min_by(|a, b| a.1.total_cmp(&b.1));
    eprintln!(
        "{} levers after the load; lever {lever_index} at {lever_at}; nearest {nearest:?}",
        words.len()
    );
    let (_, word) = words
        .iter()
        .find(|(e, _)| {
            owner(&again, *e).is_some_and(|(o, _)| placed_at(&again, o).distance(lever_at) < 0.01)
        })
        .expect("the lever");
    assert_eq!(*word as u32, lever_word);
}

fn part(name: &str, parent: Option<usize>, kind: PartKind, x: f32, moved: bool) -> LivePart {
    LivePart {
        name: name.into(),
        parent,
        kind,
        world: Affine3A::from_translation(Vec3::new(x, 0.0, 0.0)),
        moved,
        cloth: None,
    }
}

/// The walk over the owned nodes: nothing moved keeps the stored records;
/// a moved body writes its live frame nested under its parent body's
/// record (written too, though unmoved); a stored record within a
/// millimetre stays; a node without an attachment is passed over with its
/// subtree; an emitter ends its siblings' records; a moved cloth writes its
/// particles; a placement tree is never rewritten.
#[test]
fn the_part_walk_follows_the_exporters_rules() {
    let stored = vec![PropAttachment::Transform {
        name: "leaf".into(),
        word: 7,
        transform: rows_of(Affine3A::from_translation(Vec3::new(1.0, 0.0, 0.0))),
        children: Vec::new(),
    }];
    let mut parts = vec![
        part("root", None, PartKind::None, 0.0, false),
        part("leaf", Some(0), PartKind::Body, 1.0005, false),
        part("knob", Some(1), PartKind::Body, 2.0, false),
        part("plain", Some(0), PartKind::None, 0.0, false),
        part("hidden", Some(3), PartKind::Body, 3.0, true),
        part("torch", Some(0), PartKind::Emitter, 0.0, false),
        part("after", Some(0), PartKind::Body, 4.0, true),
    ];
    assert_eq!(part_records(&stored, &parts), None);
    parts[2].moved = true;
    let out = part_records(&stored, &parts).expect("rewritten");
    let PropAttachment::Transform {
        name,
        word,
        transform,
        children,
    } = &out[0]
    else {
        panic!("{out:?}");
    };
    assert_eq!((name.as_str(), *word), ("leaf", 7));
    // Within a millimetre: the stored frame stays.
    assert_eq!(transform[3], [1.0, 0.0, 0.0]);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name(), "knob");
    // The plain node's subtree and everything after the emitter: nothing.
    assert_eq!(out.len(), 1);
    // A moved cloth writes its particles.
    parts[2].moved = false;
    parts.insert(
        5,
        LivePart {
            cloth: Some(vec![[1.0, 2.0, 3.0]]),
            ..part("banner", Some(0), PartKind::Cloth, 0.0, false)
        },
    );
    let out = part_records(&stored, &parts).expect("the cloth");
    assert!(matches!(
        &out[1],
        PropAttachment::MeshPositions { name, positions, .. }
            if name == "banner" && positions == &vec![[1.0, 2.0, 3.0]]
    ));
    let tree = vec![PropAttachment::Tree {
        entries: Vec::new(),
    }];
    assert_eq!(part_records(&tree, &parts), None);
}
