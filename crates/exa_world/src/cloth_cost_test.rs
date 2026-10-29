//! The hair cloth's CPU cost in a synthetic scene: fifteen hair-sized
//! cloths (300 particles each, in the particle mode, each hanging from the
//! head part of a living body) among 21 bodies of 16 parts, over a floor.
//! `cargo test -p exa_world --release cloth_cost -- --ignored --nocapture`
//! prints the tick's wall time, the stage's own counters and a checksum of
//! every particle, so a change that is meant to leave the simulation alone
//! can be checked bit for bit.

use super::*;
use exa_physics::body::PartSetup;
use exa_physics::cloth_record::ClothRecord;
use exa_physics::{Physics, StaticColliders};
use rapier3d::prelude::ColliderBuilder;

/// Hair columns and rows: 15 × 20 = 300 particles, 2 cm apart.
const COLUMNS: usize = 15;
const ROWS: usize = 20;
const BODIES: usize = 21;
const PARTS: usize = 16;
const HAIRS: usize = 15;

/// A hair strip's mesh in metres: hanging down from its top row.
fn hair_mesh() -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let mut vertices = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            vertices.push(Vec3::new(
                (column as f32 - COLUMNS as f32 * 0.5) * 0.02,
                -(row as f32) * 0.02,
                -0.1,
            ));
        }
    }
    let mut triangles = Vec::new();
    for row in 0..ROWS - 1 {
        for column in 0..COLUMNS - 1 {
            let a = row * COLUMNS + column;
            let (b, c, d) = (a + 1, a + COLUMNS, a + COLUMNS + 1);
            triangles.push([a, b, d]);
            triangles.push([a, d, c]);
        }
    }
    (vertices, triangles)
}

/// The scene: its app, the hair cloths and the head part each hangs from.
pub(super) fn hair_scene() -> (App, Vec<Entity>, Vec<Entity>) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ClothPlugin))
        .init_resource::<Assets<Mesh>>()
        .insert_resource(Physics::default())
        .insert_resource(StaticColliders::default())
        .insert_resource(ClothContacts(true))
        .insert_resource(ClothDebug {
            on: true,
            ..ClothDebug::default()
        });
    // A floor under everyone.
    let floor = vec![
        Vec3::new(-10.0, 0.0, -10.0),
        Vec3::new(10.0, 0.0, -10.0),
        Vec3::new(10.0, 0.0, 10.0),
        Vec3::new(-10.0, 0.0, 10.0),
    ];
    let collider = ColliderBuilder::trimesh(floor, vec![[0u32, 2, 1], [0, 3, 2]])
        .unwrap()
        .build();
    let handle = app
        .world_mut()
        .resource_mut::<Physics>()
        .0
        .insert_collider(collider, None);
    app.world_mut()
        .resource_mut::<StaticColliders>()
        .insert_aabb(
            handle.into_raw_parts().0,
            Vec3::new(-10.0, 0.0, -10.0),
            Vec3::new(10.0, 0.0, 10.0),
            0,
            exa_physics::sector::MASK_SMALL,
        );
    let (vertices, triangles) = hair_mesh();
    let pins: Vec<(usize, f32)> = (0..COLUMNS).map(|i| (i, cloth::FIRM_PIN)).collect();
    let mut hairs = Vec::new();
    let mut heads = Vec::new();
    for body in 0..BODIES {
        // Bodies stand in a 5 m square, so every hair's nine cells hold
        // every part.
        let at = Vec3::new(
            (body % 5) as f32 * 1.0 + 0.3,
            0.0,
            (body / 5) as f32 * 1.0 + 0.3,
        );
        let entities: Vec<Entity> = (0..PARTS)
            .map(|_| app.world_mut().spawn_empty().id())
            .collect();
        let setups: Vec<PartSetup> = (0..PARTS)
            .map(|part| {
                let height = 1.7 - part as f32 * 0.1;
                PartSetup {
                    bone: part,
                    entity: entities[part],
                    parent: None,
                    world: Affine3A::from_translation(at + Vec3::new(0.0, height, 0.0)),
                    centre: Vec3::ZERO,
                    half: Vec3::splat(0.08),
                    origin: Vec3::ZERO,
                    links: vec![],
                    foot: None,
                    pins: Vec::new(),
                    spheres: Some(vec![(Vec3::ZERO, 0.09), (Vec3::new(0.0, 0.0, -0.08), 0.06)]),
                }
            })
            .collect();
        app.world_mut().spawn(Ragdoll::new(&setups));
        let head = entities[0];
        let head_at = at + Vec3::new(0.0, 1.7, 0.0);
        app.world_mut()
            .entity_mut(head)
            .insert(GlobalTransform::from_translation(head_at));
        if body >= HAIRS {
            continue;
        }
        let mut hair = cloth_of(&vertices, &triangles, &pins);
        hair.record = ClothRecord::HAIR;
        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vertices.iter().map(|v| v.to_array()).collect::<Vec<_>>(),
        );
        mesh.insert_indices(bevy::mesh::Indices::U32(
            triangles.iter().flatten().map(|&i| i as u32).collect(),
        ));
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        let entity = app
            .world_mut()
            .spawn((
                ClothInstance::new(hair, mesh, (0..vertices.len()).collect()),
                GlobalTransform::from_translation(head_at),
                ChildOf(head),
            ))
            .id();
        hairs.push(entity);
        heads.push(head);
    }
    (app, hairs, heads)
}

/// Turns each head a little about y, as a frame's propagation would.
pub(super) fn turn_heads(app: &mut App, heads: &[Entity], frame: usize) {
    for (i, &head) in heads.iter().enumerate() {
        let mut at = app.world_mut().get_mut::<GlobalTransform>(head).unwrap();
        let place = at.translation();
        let angle = ((frame + i) as f32 * 0.05).sin() * 0.6;
        *at = GlobalTransform::from(
            Transform::from_translation(place).with_rotation(Quat::from_rotation_y(angle)),
        );
    }
}

/// Every particle's position and last position, folded bit by bit.
pub(super) fn checksum(app: &App, hairs: &[Entity]) -> u64 {
    let mut sum = 0xcbf2_9ce4_8422_2325u64;
    for &hair in hairs {
        let instance = app.world().get::<ClothInstance>(hair).unwrap();
        for p in &instance.cloth.particles {
            for v in [p.position, p.previous] {
                for x in v.to_array() {
                    sum = (sum ^ u64::from(x.to_bits())).wrapping_mul(0x100_0000_01b3);
                }
            }
            sum = (sum ^ instance.cloth.sleep as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    sum
}

/// Fifteen awake hairs among 336 parts: the tick's wall time, the stage's
/// counted time and the frame's mesh write-back, with the checksum after
/// 400 ticks (100 frames of four ticks, the heads turning each frame).
#[test]
#[ignore = "a timing bench: run with --release --ignored --nocapture"]
fn cloth_cost_of_fifteen_hairs() {
    let (mut app, hairs, heads) = hair_scene();
    let mut tick_micros = Vec::new();
    let mut frame_micros = Vec::new();
    let mut counted = 0u64;
    for frame in 0..100 {
        turn_heads(&mut app, &heads, frame);
        for _ in 0..4 {
            let started = std::time::Instant::now();
            app.world_mut().run_schedule(FixedUpdate);
            tick_micros.push(started.elapsed().as_micros() as u64);
            counted += app.world().resource::<ClothDebug>().last_micros;
        }
        let started = std::time::Instant::now();
        app.world_mut()
            .run_system_cached(write_cloth_meshes)
            .unwrap();
        frame_micros.push(started.elapsed().as_micros() as u64);
    }
    let ticks = tick_micros.len() as f64;
    let mean = tick_micros.iter().sum::<u64>() as f64 / ticks / 1000.0;
    let mut sorted = tick_micros.clone();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2] as f64 / 1000.0;
    let frame = frame_micros.iter().sum::<u64>() as f64 / frame_micros.len() as f64 / 1000.0;
    let debug = app.world().resource::<ClothDebug>();
    let last = debug
        .cloths
        .iter()
        .map(|c| c.1)
        .fold(ClothWork::default(), |mut a, w| {
            a.add(&w);
            a
        });
    println!(
        "cloth cost: tick {mean:.3} ms mean, {median:.3} ms median, counted {:.3} ms; frame write-back {frame:.3} ms; awake {}, parts {}, bodies {}, particles {}",
        counted as f64 / ticks / 1000.0,
        last.awake,
        last.parts,
        last.bodies,
        last.particles,
    );
    println!("cloth checksum: {:016x}", checksum(&app, &hairs));
    assert_eq!(last.awake as usize, HAIRS);
}

/// The cloths step at once, and none reads another, so each hair ends
/// bit for bit where it ends stepped alone (its fellows despawned).
#[test]
fn each_cloth_steps_as_it_would_alone() {
    let run = |keep: Option<usize>| {
        let (mut app, hairs, heads) = hair_scene();
        if let Some(keep) = keep {
            for (i, &hair) in hairs.iter().enumerate() {
                if i != keep {
                    app.world_mut().despawn(hair);
                }
            }
        }
        for frame in 0..10 {
            turn_heads(&mut app, &heads, frame);
            for _ in 0..4 {
                app.world_mut().run_schedule(FixedUpdate);
            }
        }
        let kept: Vec<Entity> = match keep {
            Some(keep) => vec![hairs[keep]],
            None => hairs.clone(),
        };
        kept.iter()
            .map(|&hair| checksum(&app, &[hair]))
            .collect::<Vec<u64>>()
    };
    let together = run(None);
    for keep in [0, 7, HAIRS - 1] {
        assert_eq!(run(Some(keep)), [together[keep]], "hair {keep}");
    }
}

/// The ticks leave the meshes alone; the frame's write-back then gives
/// each mesh its particles' places in metres, once.
#[test]
fn the_meshes_take_the_particles_once_a_frame() {
    let (mut app, hairs, heads) = hair_scene();
    turn_heads(&mut app, &heads, 1);
    let positions = |app: &App, hair: Entity| {
        let instance = app.world().get::<ClothInstance>(hair).unwrap();
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&instance.mesh).unwrap();
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap()
            .to_vec()
    };
    let at_rest = positions(&app, hairs[0]);
    for _ in 0..4 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(positions(&app, hairs[0]), at_rest);
    let instance = app.world().get::<ClothInstance>(hairs[0]).unwrap();
    assert!(instance.stale_mesh);
    app.world_mut()
        .run_system_cached(write_cloth_meshes)
        .unwrap();
    let instance = app.world().get::<ClothInstance>(hairs[0]).unwrap();
    assert!(!instance.stale_mesh);
    let expected: Vec<[f32; 3]> = instance
        .vertex_particle
        .iter()
        .map(|&p| (instance.cloth.particles[p].position * 0.01).to_array())
        .collect();
    assert_ne!(expected, at_rest);
    assert_eq!(positions(&app, hairs[0]), expected);
}
