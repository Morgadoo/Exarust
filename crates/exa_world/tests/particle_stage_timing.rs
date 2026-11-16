//! A debug timing of the particles' body pass in the solver (ignored by
//! default): 120 linked three-part chains with box node meshes dropped in a
//! heap on a floor, with the pair stage, and four fires' emitters (each run
//! 300 ticks first, so their rings are full) staged in the particles'
//! stage among them. Prints the mean CPU time a tick without the stage, with
//! the stage and nothing staged, and with the four emitters. Run with
//! `cargo test --release -p exa_world --test particle_stage_timing -- --ignored --nocapture`.
use std::time::Instant;

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_physics::body::{Pairs, PartSetup, Ragdoll, SolverStage, solve_with};
use exa_world::particle_stage::{ParticleSolve, StagedEmitter};
use exa_world::particles::{Emitter, NodeFrame, Rng};

fn cube(h: Vec3) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for axis in 0..3 {
        for sign in [-1.0f32, 1.0] {
            let n = Vec3::AXES[axis] * sign;
            let (u, v) = (Vec3::AXES[(axis + 1) % 3], Vec3::AXES[(axis + 2) % 3]);
            let (u, v) = if sign > 0.0 { (u, v) } else { (v, u) };
            let c = n * h;
            let (a, b) = (c - u * h - v * h, c + u * h - v * h);
            let (d, e) = (c + u * h + v * h, c - u * h + v * h);
            out.push([a, b, d]);
            out.push([a, d, e]);
        }
    }
    out
}

fn chain(id: u32, at: Vec3) -> Ragdoll {
    let half = Vec3::new(0.12, 0.05, 0.05);
    let parts: Vec<PartSetup> = (0..3)
        .map(|k| PartSetup {
            bone: k + 2,
            entity: Entity::from_raw_u32(id * 4 + k as u32 + 1).unwrap(),
            parent: k.checked_sub(1),
            world: Affine3A::from_translation(at + Vec3::X * (0.25 * k as f32)),
            centre: Vec3::ZERO,
            half,
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
    let mut body = Ragdoll::new(&parts);
    for k in 0..3 {
        body.set_part_triangles(k, cube(half));
    }
    body
}

fn fire(at_cm: Vec3) -> Emitter {
    let spec = exa_world::fire::preset(0.8).flame;
    let mut emitter = Emitter::new(spec.template, spec.size, Vec::new());
    let node = NodeFrame {
        position: at_cm,
        rotation: Mat3::IDENTITY,
    };
    let mut rng = Rng::new(7);
    for _ in 0..300 {
        emitter.tick(node, &mut rng);
    }
    emitter
}

#[test]
#[ignore]
fn particle_stage_tick_time() {
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
    let heap = || -> Vec<Ragdoll> {
        (0..120)
            .map(|i| {
                let (x, z, y) = (
                    (i % 6) as f32 * 0.4,
                    ((i / 6) % 4) as f32 * 0.3,
                    (i / 24) as f32 * 0.2,
                );
                chain(i, Vec3::new(x, 0.3 + y, z))
            })
            .collect()
    };
    let fires = || -> Vec<StagedEmitter> {
        [(0.5, 0.4), (1.3, 0.2), (2.1, 0.8), (0.9, 1.0)]
            .iter()
            .enumerate()
            .map(|(i, &(x, z))| StagedEmitter {
                entity: Entity::from_raw_u32(1000 + i as u32).unwrap(),
                emitter: fire(Vec3::new(x * 100.0, 10.0, z * 100.0)),
                parent: None,
                contacts: true,
            })
            .collect()
    };
    let run = |label: &str, stage: Option<ParticleSolve>| {
        let mut bodies = heap();
        let mut stages: Vec<Box<dyn SolverStage>> = vec![Box::new(Pairs::default())];
        if let Some(stage) = stage {
            stages.push(Box::new(stage));
        }
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
        println!("{label}: {each:?} a tick for {} bodies", bodies.len());
    };
    run("no particle stage", None);
    run("stage, nothing staged", Some(ParticleSolve::default()));
    let mut live = ParticleSolve::new(fires(), Default::default());
    let particles: usize = live.staged.iter().map(|s| s.emitter.live()).sum();
    println!("four fires, {particles} live particles");
    run("stage, four fires", Some(std::mem::take(&mut live)));
}
