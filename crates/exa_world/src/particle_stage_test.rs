//! The particles' body pass in the solver's object stage, with the real
//! physics plugin's solver.
use super::*;
use crate::particles::{Descriptor, NodeFrame, Seed, Source};
use exa_physics::body::{PartSetup, SolverStages};

/// A cube's twelve triangles of half extent `h` (m), wound outwards.
fn cube(h: f32) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for axis in 0..3 {
        for sign in [-1.0f32, 1.0] {
            let n = Vec3::AXES[axis] * sign;
            let u = Vec3::AXES[(axis + 1) % 3];
            let v = Vec3::AXES[(axis + 2) % 3];
            let (u, v) = if sign > 0.0 { (u, v) } else { (v, u) };
            let c = n * h;
            let a = c - u * h - v * h;
            let b = c + u * h - v * h;
            let d = c + u * h + v * h;
            let e = c - u * h + v * h;
            out.push([a, b, d]);
            out.push([a, d, e]);
        }
    }
    out
}

/// A one-part body (a lumbar part, skin bone 7) at `at` (m) whose node
/// mesh is a 20 cm cube.
fn body(app: &mut App, at: Vec3) -> Ragdoll {
    let node = app.world_mut().spawn_empty().id();
    let mut body = Ragdoll::new(&[PartSetup {
        bone: 8,
        entity: node,
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: Some(vec![(Vec3::ZERO, 0.1)]),
    }]);
    body.set_part_triangles(0, cube(0.1));
    body
}

/// An emitter of the small fire's template whose one particle stands
/// `gap` cm off the cube's +x face of a body at the origin, aged so its
/// radius is `radius` cm; dose 0.5 a particle.
fn flame(gap: f32, radius: f32) -> Emitter {
    let template: Descriptor = crate::fire::SMALL.scaled(0.2);
    let mut emitter = Emitter::new(template, 0.2, vec![Source::at_node()]);
    emitter.enter(NodeFrame::default());
    emitter.params.dose = 0.5;
    let start = emitter.params.start_life;
    let rate = emitter.params.collision_growth / start as f32;
    let age = (radius / rate).ceil() as i32;
    assert!(0 < age && age < start, "{age} of {start}");
    let at = Vec3::new(10.0 + gap, 0.0, 0.0);
    emitter.seed(&[Seed {
        at,
        before: at,
        life: start - age,
        variant: 0,
    }]);
    emitter
}

fn staged(emitter: Emitter, parent: Option<Entity>) -> ParticleSolve {
    ParticleSolve {
        staged: vec![StagedEmitter {
            entity: Entity::PLACEHOLDER,
            emitter,
            parent,
            contacts: true,
        }],
        ..ParticleSolve::default()
    }
}

/// A particle whose radius reaches the part's face is pushed off the face
/// to its radius, loses a tick of life and takes −0.98 of the push into its
/// velocity; the part takes one hit's dose.
#[test]
fn the_pass_pushes_a_particle_off_a_parts_face_and_doses_the_part() {
    let mut app = App::new();
    let mut target = body(&mut app, Vec3::ZERO);
    let mut stage = staged(flame(2.0, 5.0), None);
    let before = stage.staged[0].emitter.particles().next().unwrap();
    stage.contact(&[&mut target], &[]);
    let (slot, at, q, life) = stage.staged[0].emitter.particles().next().unwrap();
    assert_eq!(slot, before.0);
    assert_eq!(life, before.3 - 1);
    let radius = {
        let p = &stage.staged[0].emitter.params;
        (p.start_life - before.3) as f32 * (p.collision_growth / p.start_life as f32)
    };
    assert!((at.x - (10.0 + radius)).abs() < 1e-4, "{at}");
    assert!((q - before.2 - (before.1 - at) * -0.98).length() < 1e-4);
    let doses = stage.take_doses();
    assert_eq!(
        doses,
        vec![Dose {
            part: target.parts[0].entity,
            skin_bone: 7,
            dose: 0.5,
        }]
    );
}

/// Every part handed over takes the slot's dose call, a part no particle
/// met with a dose of 0; the part on the emitter's own parent node, a
/// disabled body and a part far outside the nine cells are not handed
/// over.
#[test]
fn the_pass_skips_the_own_part_a_disabled_body_and_the_far_cells() {
    let mut app = App::new();
    let mut near = body(&mut app, Vec3::ZERO);
    // A particle 2 cm off the face with a 1 cm radius: the part is handed
    // over (the dose slot is called), the particle meets nothing.
    let mut stage = staged(flame(2.0, 1.0), None);
    stage.contact(&[&mut near], &[]);
    let doses = stage.take_doses();
    assert_eq!(doses.len(), 1);
    assert_eq!(doses[0].dose, 0.0);
    // 30 cm off: the emitter's bound sphere is apart from the part's.
    let mut stage = staged(flame(30.0, 5.0), None);
    stage.contact(&[&mut near], &[]);
    assert!(stage.take_doses().is_empty());
    let mut stage = staged(flame(2.0, 5.0), Some(near.parts[0].entity));
    stage.contact(&[&mut near], &[]);
    assert!(stage.take_doses().is_empty());
    let mut off = body(&mut app, Vec3::ZERO);
    off.set_disabled(true);
    let mut stage = staged(flame(2.0, 5.0), None);
    stage.contact(&[&mut off], &[]);
    assert!(stage.take_doses().is_empty());
    // A sleeping body is handed over as an awake one is.
    let mut asleep = body(&mut app, Vec3::ZERO);
    let mut stage = staged(flame(2.0, 5.0), None);
    stage.contact(&[], &[&mut asleep]);
    assert_eq!(stage.take_doses().len(), 1);
    // A body 20 m away is in none of the nine cells.
    let mut far = body(&mut app, Vec3::new(20.0, 0.0, 0.0));
    let mut stage = staged(flame(2.0, 5.0), None);
    stage.contact(&[&mut far], &[]);
    assert!(stage.take_doses().is_empty());
    // An emitter without the class's contact bit takes no pass.
    let mut stage = staged(flame(2.0, 5.0), None);
    stage.staged[0].contacts = false;
    stage.contact(&[&mut near], &[]);
    assert!(stage.take_doses().is_empty());
}

/// Inside the real solver: the stage's hook runs at the object stage of a
/// fixed step with a body falling under gravity, and the particle meets
/// the part where the solver has it then.
#[test]
fn the_stage_runs_in_the_solvers_object_stage() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, exa_physics::PhysicsPlugin));
    let target = body(&mut app, Vec3::ZERO);
    let part = target.parts[0].entity;
    app.world_mut().spawn(target);
    app.world_mut()
        .resource_mut::<SolverStages>()
        .add(staged(flame(2.0, 5.0), None));
    app.world_mut().run_schedule(FixedUpdate);
    let mut stages = app.world_mut().resource_mut::<SolverStages>();
    let stage = stages.get_mut::<ParticleSolve>().unwrap();
    let doses = stage.take_doses();
    assert_eq!(doses.len(), 1);
    assert_eq!((doses[0].part, doses[0].dose), (part, 0.5));
}
