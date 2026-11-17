//! Ports registered by the collision2 lane (batch g): a mesh's query slots
//! over its face records, in the brute-force form (every face in order) and
//! the kd-tree leaf form (a list of face ids). The case lays out six face
//! records over eighteen corners, the query, and a callback that logs each
//! call (face, closest point, push); the port runs the per-face test of
//! [`exa_physics::mesh_query`] (or the cloth's sweep in
//! [`exa_physics::cloth_level`]) over the same faces and writes the same
//! log.

use bevy::math::Vec3;
use exa_physics::cloth_level;
use exa_physics::mesh_query::{self as mq, Face, Swept};

use super::{Case, Out, Port};

const FACES: usize = 6;
const LOG: usize = 10;
const INDICES: usize = 8;

fn i32_of(b: &Case, name: &str) -> i32 {
    let f = b.field(name);
    i32::from_le_bytes([f[0], f[1], f[2], f[3]])
}

fn corner(b: &Case, k: usize) -> Vec3 {
    Vec3::new(
        b.f32(&format!("verts.v{k}x")),
        b.f32(&format!("verts.v{k}y")),
        b.f32(&format!("verts.v{k}z")),
    )
}

/// The case's face records as they are (not rebuilt from the corners).
fn faces(b: &Case) -> Vec<Face> {
    (0..FACES)
        .map(|i| {
            let f = |n: &str| b.f32(&format!("faces.f{i}{n}"));
            Face {
                vertices: [corner(b, 3 * i), corner(b, 3 * i + 1), corner(b, 3 * i + 2)],
                normal: Vec3::new(f("nx"), f("ny"), f("nz")),
                plane: f("plane"),
                edge_squared: f("e2"),
                inverse_gram: f("ig"),
            }
        })
        .collect()
}

/// The faces a brute-force query visits: 0 to the mesh's last index.
fn every(b: &Case) -> Vec<usize> {
    let last = i32_of(b, "mesh.last");
    (0..=last).map(|i| i as usize).collect()
}

/// The faces a leaf visits: the first `count` ids of the list.
fn listed(b: &Case, count: i32) -> Vec<usize> {
    (0..count.clamp(0, INDICES as i32))
        .map(|k| i32_of(b, &format!("idx.i{k}")) as usize)
        .collect()
}

/// The callback's log: each call's face, closest point and push.
#[derive(Default)]
struct Log(Vec<(usize, Vec3, Vec3)>);

impl Log {
    fn out(&self) -> Out {
        let mut out = Out::default().i32(self.0.len() as i32);
        for k in 0..LOG {
            let (face, q, p) = self
                .0
                .get(k)
                .copied()
                .unwrap_or((0, Vec3::ZERO, Vec3::ZERO));
            out = out.i32(face as i32).vec3(q).vec3(p);
        }
        out
    }
}

fn sphere(b: &Case) -> (Vec3, f32) {
    (b.vec3("q"), b.f32("q.r"))
}

fn two_sided(b: &Case, ids: Vec<usize>) -> Vec<u8> {
    let faces = faces(b);
    let (centre, r) = sphere(b);
    let mut log = Log::default();
    for i in ids {
        if let Some(q) = mq::two_sided_face(&faces[i], centre, r, mq::SMALL) {
            log.0.push((i, q, Vec3::ZERO));
        }
    }
    let any = !log.0.is_empty();
    log.out().bytes(&[any as u8]).0
}

fn front(b: &Case, ids: Vec<usize>, floor: f32) -> Vec<u8> {
    let faces = faces(b);
    let (centre, r) = sphere(b);
    let mut log = Log::default();
    for i in ids {
        if let Some((q, push)) = mq::front_push_face(&faces[i], centre, r, floor) {
            log.0.push((i, q, push));
        }
    }
    let any = !log.0.is_empty();
    log.out().bytes(&[any as u8]).0
}

/// The sweep record's fields.
fn record(b: &Case) -> Swept {
    let v = |p: &str| {
        Vec3::new(
            b.f32(&format!("s.{p}x")),
            b.f32(&format!("s.{p}y")),
            b.f32(&format!("s.{p}z")),
        )
    };
    Swept {
        start: v("s"),
        end: v("e"),
        radius: b.f32("s.r"),
        direction: v("d"),
        length: b.f32("s.length"),
    }
}

/// The swept or cloth sweep query over `ids`: the log and the last face
/// pushed (-1 for none). The cloth's leaf hands its callback the position
/// in the list, not the face id (`by_position`); the cloth's callback does
/// not read it.
fn swept(
    b: &Case,
    ids: Vec<usize>,
    test: fn(&Face, &Swept) -> Option<(Vec3, Vec3)>,
    by_position: bool,
) -> (Log, i32) {
    let faces = faces(b);
    let s = record(b);
    let mut log = Log::default();
    let mut last = -1;
    for (k, i) in ids.into_iter().enumerate() {
        if let Some((q, push)) = test(&faces[i], &s) {
            log.0.push((if by_position { k } else { i }, q, push));
            last = i as i32;
        }
    }
    (log, last)
}

/// The swept nearest query over `ids`: the record with its length cut to
/// the nearest hit, and the last face hit.
fn nearest(b: &Case, ids: Vec<usize>) -> (Out, Option<usize>) {
    let faces = faces(b);
    let s = record(b);
    let mut best = s.length;
    let mut hit = None;
    for i in ids {
        if mq::swept_nearest(&faces[i], s.start, s.direction, s.radius, &mut best) {
            hit = Some(i);
        }
    }
    let out = Out::default()
        .bytes(&[0; 12])
        .f32(best)
        .vec3(s.start)
        .f32(s.radius)
        .vec3(s.end)
        .bytes(&[0; 4])
        .vec3(s.direction)
        .bytes(&[0; 20]);
    (out, hit)
}

pub(super) const PORTS: &[Port] = &[
    Port {
        name: "mesh_two_sided_all",
        run: |b| two_sided(b, every(b)),
    },
    Port {
        name: "mesh_two_sided_leaf",
        run: |b| two_sided(b, listed(b, i32_of(b, "count"))),
    },
    Port {
        name: "mesh_front_all",
        run: |b| front(b, every(b), mq::BRUTE_PUSH_FLOOR),
    },
    Port {
        name: "mesh_front_leaf",
        run: |b| front(b, listed(b, i32_of(b, "count")), mq::PUSH_FLOOR),
    },
    Port {
        name: "mesh_swept_all",
        run: |b| {
            let (log, last) = swept(b, every(b), mq::swept_push_face, false);
            log.out().i32(last).0
        },
    },
    Port {
        name: "mesh_swept_leaf",
        run: |b| {
            let (log, last) = swept(b, listed(b, i32_of(b, "count")), mq::swept_push_face, false);
            log.out().bytes(&[(last >= 0) as u8]).0
        },
    },
    Port {
        name: "mesh_cloth_swept_all",
        run: |b| {
            let (log, last) = swept(b, every(b), cloth_level::sweep_push, false);
            log.out().i32(last).0
        },
    },
    Port {
        name: "mesh_cloth_swept_leaf",
        run: |b| {
            let (log, last) = swept(
                b,
                listed(b, i32_of(b, "count")),
                cloth_level::sweep_push,
                true,
            );
            log.out().i32(last).0
        },
    },
    Port {
        name: "mesh_nearest_all",
        run: |b| {
            let (out, hit) = nearest(b, every(b));
            out.i32(hit.map_or(0, |i| i as i32 + 1)).0
        },
    },
    Port {
        name: "mesh_nearest_leaf",
        run: |b| {
            let last = i32_of(b, "last");
            let ids: Vec<usize> = (0..=last)
                .map(|k| i32_of(b, &format!("idx.i{k}")) as usize)
                .collect();
            let (out, hit) = nearest(b, ids);
            out.i32(hit.map_or(-1, |i| i as i32)).0
        },
    },
];
