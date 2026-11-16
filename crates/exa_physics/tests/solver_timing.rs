//! A debug timing of the solver's tick (ignored by default): 120 linked
//! three-part chains dropped in a heap on a floor, with the pair stage, so
//! the level hits, the pair hits and both friction passes all run. Prints
//! the mean CPU time a tick. Run with
//! `cargo test --release -p exa_physics --test solver_timing -- --ignored --nocapture`.
use std::time::Instant;

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_physics::body::{PartSetup, Pairs, Ragdoll, SolverStage, solve_with};

fn chain(id: u32, at: Vec3) -> Ragdoll {
    let parts: Vec<PartSetup> = (0..3)
        .map(|k| PartSetup {
            bone: k + 1,
            entity: Entity::from_raw_u32(id * 4 + k as u32 + 1).unwrap(),
            parent: k.checked_sub(1),
            world: Affine3A::from_translation(at + Vec3::X * (0.25 * k as f32)),
            centre: Vec3::ZERO,
            half: Vec3::new(0.12, 0.05, 0.05),
            origin: if k == 0 { Vec3::ZERO } else { Vec3::X * 0.25 },
            links: if k == 0 {
                vec![]
            } else {
                vec![(Vec3::X * -0.125, Vec3::X * 0.125, 0.0)]
            },
            foot: None,
            pins: Vec::new(),
            spheres: None,
        })
        .collect();
    Ragdoll::new(&parts)
}

#[test]
#[ignore]
fn solver_tick_time() {
    let floor = vec![
        [
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::new(-20.0, 0.0, 20.0),
            Vec3::new(20.0, 0.0, 20.0),
        ],
        [
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::new(20.0, 0.0, 20.0),
            Vec3::new(20.0, 0.0, -20.0),
        ],
    ];
    let mut bodies: Vec<Ragdoll> = (0..120)
        .map(|i| {
            let (x, z, y) = ((i % 6) as f32 * 0.4, ((i / 6) % 4) as f32 * 0.3, (i / 24) as f32 * 0.2);
            chain(i, Vec3::new(x, 0.3 + y, z))
        })
        .collect();
    let mut stages: Vec<Box<dyn SolverStage>> = vec![Box::new(Pairs::default())];
    let mut tick = |bodies: &mut Vec<Ragdoll>| {
        let mut refs: Vec<&mut Ragdoll> = bodies.iter_mut().collect();
        solve_with(&mut refs, floor.as_slice(), &mut stages);
    };
    for _ in 0..50 {
        tick(&mut bodies);
    }
    let ticks = 400;
    let started = Instant::now();
    for _ in 0..ticks {
        tick(&mut bodies);
    }
    let each = started.elapsed() / ticks;
    println!("solver tick: {each:?} for {} bodies", bodies.len());
}
