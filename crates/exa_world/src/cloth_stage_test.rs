//! The cloths inside the bodies' tick: the body side of the contact in the
//! solver, with the real physics and cloth plugins.
//! (Included from cloth_stage.rs: `#[cfg(test)] #[path = "cloth_stage_test.rs"] mod tests;`)
use super::*;
use crate::cloth::{ClothPlugin, cloth_of};
use exa_physics::body::PartSetup;

/// An app with the physics (bodies, solver) and the cloth plugins, its
/// cloths' contacts on and their debug lines kept.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, exa_physics::PhysicsPlugin, ClothPlugin))
        .init_resource::<Assets<Mesh>>()
        .insert_resource(ClothContacts(true))
        .insert_resource(ClothDebug {
            on: true,
            ..ClothDebug::default()
        });
    app
}

/// A one-part body with one sphere of radius `radius` (m) at `at` (m),
/// its part on a node of its own.
fn body(app: &mut App, at: Vec3, radius: f32) -> Ragdoll {
    let node = app.world_mut().spawn_empty().id();
    Ragdoll::new(&[PartSetup {
        bone: 1,
        entity: node,
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(radius * 0.5),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: Some(vec![(Vec3::ZERO, radius)]),
    }])
}

/// A level metre-square sheet at height `y` (m), its corners firmly
/// pinned, at `x` along the x axis.
fn sheet(app: &mut App, x: f32, y: f32) -> Entity {
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        ));
    let corners = [
        Vec3::new(x, y, 0.0),
        Vec3::new(x + 1.0, y, 0.0),
        Vec3::new(x + 1.0, y, 1.0),
        Vec3::new(x, y, 1.0),
    ];
    let pins: Vec<(usize, f32)> = (0..4).map(|i| (i, 1.0)).collect();
    let cloth = cloth_of(&corners, &[[0, 1, 2], [0, 2, 3]], &pins);
    let drawn = (0..cloth.particles.len()).collect();
    app.world_mut()
        .spawn((ClothInstance::new(cloth, mesh, drawn), GlobalTransform::default()))
        .id()
}

/// The body side in the solver: a sleeping body whose sphere reaches up
/// through an awake sheet is pushed down by its share of the sheet's push
/// (0.002 of it, a level cloth's word) in the same tick, inside the
/// solver's object stage, and woken by the countdown trade up to the
/// cloth's; with the contacts off it stays asleep where it was.
#[test]
fn a_cloth_pushes_and_wakes_the_body_in_the_solver() {
    let run = |contacts: bool| {
        let mut app = app();
        app.insert_resource(ClothContacts(contacts));
        let mut asleep = body(&mut app, Vec3::new(0.5, 0.95, 0.5), 0.1);
        asleep.hold_out();
        asleep.set_disabled(false);
        let before = asleep.parts[0].positions;
        let part = asleep.parts[0].entity;
        let entity = app.world_mut().spawn(asleep).id();
        let cloth = sheet(&mut app, 0.0, 1.0);
        app.world_mut().run_schedule(FixedUpdate);
        let body = app.world().get::<Ragdoll>(entity).unwrap().clone();
        let moved: Vec3 = (0..4).map(|k| body.parts[0].positions[k] - before[k]).sum();
        let debug = app.world().resource::<ClothDebug>();
        let pushers = debug
            .cloths
            .iter()
            .find(|(e, ..)| *e == cloth)
            .map(|(_, _, p)| p.clone())
            .unwrap_or_default();
        let sleep = app.world().get::<ClothInstance>(cloth).unwrap().cloth.sleep;
        (body, moved, pushers, part, sleep)
    };
    let (body, moved, pushers, part, sleep) = run(true);
    assert!(!body.asleep(), "the cloth's trade woke the body");
    assert!(body.countdown() >= 2 && sleep > 0, "{} {sleep}", body.countdown());
    assert_eq!(pushers.len(), 1);
    assert_eq!(pushers[0].0, part);
    // The sphere (10 cm, padded 12) stands 5 cm under the sheet: each of
    // the two triangles is pushed some 7 cm up, and the body takes 0.002 of
    // the summed push down, spread by weights that sum to one.
    assert!(moved.x.abs() < 1e-6 && moved.z.abs() < 1e-6, "{moved}");
    assert!(
        moved.y < -1.0e-4 && moved.y > -4.0e-4,
        "the body's share of the push: {moved}"
    );
    let (body, moved, pushers, ..) = run(false);
    assert!(body.asleep());
    assert_eq!(moved, Vec3::ZERO);
    assert!(pushers.is_empty());
}

/// The cloths meet the bodies one at a time in the object list's order, so
/// two runs of two overlapping sheets under the same falling bodies end bit
/// for bit alike, the bodies' places and the cloths' particles both (a
/// cloth's 0.002 share barely slows a body: they fall through).
#[test]
fn the_cloths_meet_the_bodies_in_a_fixed_order() {
    let run = || {
        let mut app = app();
        let bodies: Vec<Entity> = [0.5, 1.0, 1.5]
            .into_iter()
            .map(|x| {
                let b = body(&mut app, Vec3::new(x, 1.3, 0.5), 0.1);
                app.world_mut().spawn(b).id()
            })
            .collect();
        let cloths = [sheet(&mut app, 0.0, 1.0), sheet(&mut app, 0.9, 1.05)];
        let mut pushed = 0;
        for _ in 0..40 {
            app.world_mut().run_schedule(FixedUpdate);
            pushed += app
                .world()
                .resource::<ClothDebug>()
                .cloths
                .iter()
                .map(|(_, _, p)| p.len())
                .sum::<usize>();
        }
        let places: Vec<[Vec3; 4]> = bodies
            .iter()
            .map(|&e| app.world().get::<Ragdoll>(e).unwrap().parts[0].positions)
            .collect();
        let particles: Vec<Vec<Vec3>> = cloths
            .iter()
            .map(|&e| {
                let c = &app.world().get::<ClothInstance>(e).unwrap().cloth;
                c.particles.iter().map(|p| p.position).collect()
            })
            .collect();
        (places, particles, pushed)
    };
    let first = run();
    let second = run();
    let bits = |v: &[[Vec3; 4]]| -> Vec<u32> {
        v.iter()
            .flat_map(|p| p.iter().flat_map(|x| x.to_array().map(f32::to_bits)))
            .collect()
    };
    assert_eq!(bits(&first.0), bits(&second.0));
    assert_eq!(first.1, second.1);
    assert_eq!(first.2, second.2);
    assert!(first.2 >= 3, "the bodies met the sheets: {} pushes", first.2);
}
