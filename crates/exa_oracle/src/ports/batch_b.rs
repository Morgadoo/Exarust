//! Ports registered by the second oracle lane (batch b): the simulation's
//! sector, sweep and particle-body helpers.

use bevy::math::{Affine3A, Mat3A, Vec3};
use exa_physics::{body, intersect, sector};

#[allow(unused_imports)]
use super::{Case, In, Out, Port};

/// The named `u32` field.
fn u32_of(b: &Case, name: &str) -> u32 {
    let f = b.field(name);
    u32::from_le_bytes([f[0], f[1], f[2], f[3]])
}

/// Four weights in place.
fn weights4(b: &Case, f: fn([f32; 4]) -> [f32; 4]) -> Vec<u8> {
    Out::default().f32s(&f(In(b).array())).0
}

/// A 4×3 matrix of rows (x axis, y axis, z axis, translation).
fn rows_4x3(m: [f32; 12]) -> Affine3A {
    Affine3A {
        matrix3: Mat3A::from_cols(
            Vec3::new(m[0], m[1], m[2]).into(),
            Vec3::new(m[3], m[4], m[5]).into(),
            Vec3::new(m[6], m[7], m[8]).into(),
        ),
        translation: Vec3::new(m[9], m[10], m[11]).into(),
    }
}

fn matrix12(b: &Case) -> [f32; 12] {
    std::array::from_fn(|i| b.f32(&format!("m.m[{i}]")))
}

/// A point through a 4×3 matrix with its w (a sphere's radius) kept.
fn point_4x3(b: &Case, f: fn(Vec3, &Affine3A) -> Vec3) -> Vec<u8> {
    let p = f(b.vec3("p"), &rows_4x3(matrix12(b)));
    Out::default().vec3(p).f32(b.f32("p.w")).0
}

/// A sweep record: length, start and end, direction and its reciprocal
/// (the w lanes are pads).
fn sweep_new(b: &Case) -> Vec<u8> {
    let start = Vec3::new(b.f32("s.sx"), b.f32("s.sy"), b.f32("s.sz"));
    let end = Vec3::new(b.f32("s.ex"), b.f32("s.ey"), b.f32("s.ez"));
    let s = intersect::Sweep::new(start, end);
    Out::default()
        .bytes(&[0; 12])
        .f32(s.length)
        .vec3(s.origin)
        .bytes(&[0; 4])
        .vec3(end)
        .bytes(&[0; 4])
        .vec3(s.direction)
        .bytes(&[0; 4])
        .vec3(s.inverse_direction)
        .bytes(&[0; 4])
        .0
}

/// A row matrix of the sweep's pair (rows `r{first}` to `r{first + 3}`).
fn rows_4x4(b: &Case, first: usize) -> Affine3A {
    let row = |i: usize| {
        Vec3::new(
            b.f32(&format!("m.r{i}x")),
            b.f32(&format!("m.r{i}y")),
            b.f32(&format!("m.r{i}z")),
        )
    };
    Affine3A {
        matrix3: Mat3A::from_cols(
            row(first).into(),
            row(first + 1).into(),
            row(first + 2).into(),
        ),
        translation: row(first + 3).into(),
    }
}

/// The pair test's sweep set-up in the original's units: the sphere centre
/// through both frames (glam's `transform_point3`, as the pair test's port
/// takes it), the move gated at 0.1 squared, then [`intersect::Sweep::of_move`].
/// Fields the original leaves alone keep their input bytes.
fn sweep_between(b: &Case) -> Vec<u8> {
    let p = b.vec3("p");
    let end = rows_4x4(b, 0).transform_point3(p);
    let start = rows_4x4(b, 4).transform_point3(p);
    // The record is the first argument, 0x50 bytes.
    let mut rec = b[..0x50].to_vec();
    let mut put = |at: usize, v: Vec3| {
        for (k, x) in v.to_array().into_iter().enumerate() {
            rec[at + 4 * k..at + 4 * k + 4].copy_from_slice(&x.to_le_bytes());
        }
    };
    let swept = (end - start).length_squared() > 0.1;
    if !swept {
        put(0x10, end);
    } else {
        let s = intersect::Sweep::of_move(start, end);
        put(0x10, start);
        put(0x20, end);
        put(0x30, s.direction);
        put(0x40, s.inverse_direction);
        rec[0xc..0x10].copy_from_slice(&s.length.to_le_bytes());
    }
    Out::default()
        .bytes(&rec)
        .bytes(&u32::from(swept).to_le_bytes())
        .0
}

/// A body's damage call: its bars after `Health::take`, the rate reset,
/// and the weaken or kill call's byte (the case's stubs mark them).
fn health_take(b: &Case, story: bool) -> Vec<u8> {
    use exa_combat::condition::StateFlags;
    use exa_combat::fighter::{Damage, Health, Taken};
    let mut health = Health {
        yellow: b.f32("body.yellow"),
        red: b.f32("body.red"),
        max: b.f32("body.max"),
        rate: b.f32("body.rate"),
        heal_pool: 0.0,
    };
    let damage = Damage {
        yellow: b.f32("yellow"),
        red: b.f32("red"),
        vital: b.u8("vital") != 0,
    };
    let rules = exa_formats::character_stats::BodyRules(u32_of(b, "body.rules"));
    let taken = health.take(damage, rules, StateFlags(u32_of(b, "body.flags")), story);
    // The body is the first argument, 0x1320 bytes.
    let mut rec = b[..0x1320].to_vec();
    let mut put = |at: usize, v: f32| rec[at..at + 4].copy_from_slice(&v.to_le_bytes());
    put(0xd58, health.yellow);
    put(0xd5c, health.red);
    put(0x1010, health.rate);
    rec[0x1319] = u8::from(taken == Taken::Weakened);
    rec[0x131a] = u8::from(taken == Taken::Killed);
    Out::default()
        .bytes(&rec)
        .bytes(&[u8::from(taken == Taken::Killed)])
        .0
}

pub(super) const PORTS: &[Port] = &[
    Port {
        name: "health_take",
        run: |b| health_take(b, false),
    },
    Port {
        name: "health_take_story",
        run: |b| health_take(b, true),
    },
    Port {
        name: "sweep_between",
        run: sweep_between,
    },
    Port {
        name: "scale_to_max",
        run: |b| weights4(b, |w| sector::scale_to_max(w).0),
    },
    Port {
        name: "normalise_sum",
        run: |b| weights4(b, |w| sector::normalise_sum(w).expect("a positive weight")),
    },
    Port {
        name: "swept_box_hit",
        run: |b| {
            let hit = sector::swept_box_hit(
                Vec3::new(b.f32("seg.sx"), b.f32("seg.sy"), b.f32("seg.sz")),
                Vec3::new(b.f32("seg.ex"), b.f32("seg.ey"), b.f32("seg.ez")),
                b.f32("seg.radius"),
                b.vec3("centre"),
                b.vec3("half"),
            );
            u32::from(hit).to_le_bytes().to_vec()
        },
    },
    Port {
        name: "segment_box_slabs",
        run: |b| {
            let hit = intersect::segment_box_slabs(
                Vec3::new(b.f32("seg.sx"), b.f32("seg.sy"), b.f32("seg.sz")),
                Vec3::new(b.f32("seg.dx"), b.f32("seg.dy"), b.f32("seg.dz")),
                b.f32("seg.length"),
                Vec3::new(b.f32("box.cx"), b.f32("box.cy"), b.f32("box.cz")),
                Vec3::new(b.f32("box.hx"), b.f32("box.hy"), b.f32("box.hz")),
            );
            vec![u8::from(hit)]
        },
    },
    // The engine's form (the original's sum order); glam's
    // `transform_point3` differs in 1973 of 5000 cases (last bit).
    Port {
        name: "point_4x3_keep_w",
        run: |b| point_4x3(b, exa_physics::engine_math::point_out_of_frame),
    },
    Port {
        name: "sweep_new",
        run: sweep_new,
    },
    Port {
        name: "restore_edge",
        run: |b| {
            let (mut a, mut c) = (b.vec3("a"), b.vec3("b"));
            body::restore_edge(&mut a, &mut c, b.f32("rest"));
            Out::default().vec3(a).vec3(c).0
        },
    },
    Port {
        name: "push_weighted",
        run: |b| {
            let mut i = In(b);
            let flat: [f32; 12] = i.array();
            let delta = i.vec3();
            let w: [f32; 4] = i.array();
            let scale = i.f32();
            let mut p: [Vec3; 4] =
                std::array::from_fn(|k| Vec3::new(flat[3 * k], flat[3 * k + 1], flat[3 * k + 2]));
            body::push_weighted(&mut p, &w, delta, scale);
            Out::default().vec3(p[0]).vec3(p[1]).vec3(p[2]).vec3(p[3]).0
        },
    },
    Port {
        name: "clock_fraction",
        run: |b| {
            let f = body::clock_fraction(
                u32_of(b, "clock.now"),
                u32_of(b, "clock.stamp"),
                u32_of(b, "clock.period"),
                b.f32("offset"),
                b.f32("clock.scale"),
            );
            Out::default().f32(f).0
        },
    },
    Port {
        name: "mobility_shares",
        run: |b| {
            let (own, other) = body::mobility_shares(b.f32("child.m"), b.f32("parent.m"));
            Out::default()
                .bytes(&[0; 0x44])
                .f32(own)
                .f32(-other)
                .bytes(&[0; 4])
                .0
        },
    },
];
