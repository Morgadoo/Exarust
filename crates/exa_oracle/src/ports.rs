//! The registry: each original function's name and a closure that runs its
//! Rust port on a case's input bytes. A closure reads its inputs in the
//! case's field order (the original's arguments in call order, a buffer
//! passed by address as its bytes) and writes its outputs in the case's
//! order: each out buffer, then the return value.

use bevy::math::{Quat, Vec2, Vec3, Vec4};
use exa_physics::engine_math as em;

use crate::Field;

/// One registered port.
pub struct Port {
    /// The function's name, as its case files carry it.
    pub name: &'static str,
    /// A case's inputs to its output bytes through the port.
    pub run: fn(&Case) -> Vec<u8>,
}

/// One case's input bytes with the file's input fields, so that a port can
/// read a field by its name wherever the layout puts it.
pub struct Case<'a> {
    pub bytes: &'a [u8],
    pub fields: &'a [Field],
}

impl std::ops::Deref for Case<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.bytes
    }
}

impl Case<'_> {
    /// The bytes of the named input field.
    pub fn field(&self, name: &str) -> &[u8] {
        let mut at = 0;
        for f in self.fields {
            let n = f.size();
            if f.name == name {
                return &self.bytes[at..at + n];
            }
            at += n;
        }
        panic!("no input field {name}");
    }

    /// The named `f32` field.
    pub fn f32(&self, name: &str) -> f32 {
        let b = self.field(name);
        f32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }

    /// The named byte field.
    pub fn u8(&self, name: &str) -> u8 {
        self.field(name)[0]
    }

    /// The named fields `<prefix>.x`, `.y` and `.z`.
    pub fn vec3(&self, prefix: &str) -> Vec3 {
        Vec3::new(
            self.f32(&format!("{prefix}.x")),
            self.f32(&format!("{prefix}.y")),
            self.f32(&format!("{prefix}.z")),
        )
    }
}

/// A little-endian reader over a case's input bytes.
struct In<'a>(&'a [u8]);

impl In<'_> {
    fn bytes<const N: usize>(&mut self) -> [u8; N] {
        let (head, rest) = self.0.split_at(N);
        self.0 = rest;
        head.try_into().expect("N bytes")
    }
    fn f32(&mut self) -> f32 {
        f32::from_le_bytes(self.bytes())
    }
    fn skip(&mut self, n: usize) {
        self.0 = &self.0[n..];
    }
    fn vec2(&mut self) -> Vec2 {
        Vec2::new(self.f32(), self.f32())
    }
    fn vec3(&mut self) -> Vec3 {
        Vec3::new(self.f32(), self.f32(), self.f32())
    }
    fn vec4(&mut self) -> Vec4 {
        Vec4::new(self.f32(), self.f32(), self.f32(), self.f32())
    }
    fn array<const N: usize>(&mut self) -> [f32; N] {
        std::array::from_fn(|_| self.f32())
    }
}

/// The output bytes, written in field order.
#[derive(Default)]
struct Out(Vec<u8>);

impl Out {
    fn f32(mut self, v: f32) -> Self {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn i32(mut self, v: i32) -> Self {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn f32s(mut self, v: &[f32]) -> Self {
        for x in v {
            self.0.extend(x.to_le_bytes());
        }
        self
    }
    fn vec2(self, v: Vec2) -> Self {
        self.f32s(&v.to_array())
    }
    fn vec3(self, v: Vec3) -> Self {
        self.f32s(&v.to_array())
    }
    fn bytes(mut self, v: &[u8]) -> Self {
        self.0.extend(v);
        self
    }
}

fn in_place3(b: &[u8], f: fn(Vec3) -> Vec3) -> Vec<u8> {
    Out::default().vec3(f(In(b).vec3())).0
}

/// An out vector first (skipped), then the inputs.
fn into3_2(b: &[u8], f: fn(Vec3, Vec3) -> Vec3) -> Vec<u8> {
    let mut i = In(b);
    i.skip(12);
    let (a, c) = (i.vec3(), i.vec3());
    Out::default().vec3(f(a, c)).0
}

fn lerp3_into(b: &[u8], f: fn(Vec3, Vec3, f32) -> Vec3) -> Vec<u8> {
    let mut i = In(b);
    i.skip(12);
    let (a, c, t) = (i.vec3(), i.vec3(), i.f32());
    Out::default().vec3(f(a, c, t)).0
}

fn scalar(b: &[u8], f: fn(f32) -> f32) -> Vec<u8> {
    Out::default().f32(f(In(b).f32())).0
}

/// The colour lerp through the particle keys' sampler: two keys, sampled
/// at the fraction `t` (the cases keep `|t| < 1`, so the first key is `a`).
fn rgba8_lerp(b: &Case) -> Vec<u8> {
    let mut i = In(b);
    i.skip(4);
    let (a, c, t): ([u8; 4], [u8; 4], f32) = (i.bytes(), i.bytes(), i.f32());
    let entries: &'static [[u8; 4]] = Box::leak(Box::new([a, c]));
    let keys = exa_world::particles::Keys { count: 2, entries };
    Out::default().bytes(&keys.sample(t, false)).0
}

/// A face record's bytes: normal, plane, three corner references (not
/// compared), the first edge squared and the inverse Gram determinant.
fn face_record(face: &exa_physics::volume::Face) -> Out {
    Out::default()
        .vec3(face.normal)
        .f32(face.plane)
        .bytes(&[0; 24])
        .f32(face.edge_squared)
        .f32(face.inverse_gram)
}

fn face_new(b: &Case) -> Vec<u8> {
    let vertices = [b.vec3("v0"), b.vec3("v1"), b.vec3("v2")];
    face_record(&exa_physics::volume::Face::new(vertices)).0
}

fn face_nearest(b: &Case) -> Vec<u8> {
    let face = exa_physics::volume::Face {
        vertices: [b.vec3("v0"), b.vec3("v1"), b.vec3("v2")],
        normal: Vec3::new(b.f32("face.nx"), b.f32("face.ny"), b.f32("face.nz")),
        plane: b.f32("face.plane"),
        edge_squared: b.f32("face.edge_squared"),
        inverse_gram: b.f32("face.inverse_gram"),
    };
    Out::default()
        .vec3(face.nearest(b.vec3("p"), b.f32("distance")))
        .bytes(&[0; 4])
        .0
}

/// A door leaf's openness: its sliding byte, its angles and its slider
/// record's travel.
fn door_open_span(b: &Case) -> Vec<u8> {
    let leaf = exa_physics::door::SignalLeaf {
        sliding: b.u8("leaf.sliding") != 0,
        latched: false,
        motorised: false,
        closed: false,
        count: 0,
        fraction: b.f32("records.fraction"),
        angle: b.f32("leaf.angle"),
        closed_angle: b.f32("leaf.closed_angle"),
        fast: false,
        drive: exa_physics::door::SignalDrive::Idle,
        countdown: 0,
    };
    Out::default().f32(exa_physics::door::open_span(&leaf)).0
}

/// A swing's expected milliseconds: the swing record's speed and
/// modulation, the fighter's progress and clock base, the clip's last key.
fn swing_expected_ms(b: &Case) -> Vec<u8> {
    let swing = exa_combat::swing::Swing {
        speed: b.f32("record.speed"),
        modulation: b.f32("record.modulation"),
        ..exa_combat::swing::Swing::FISTS
    };
    let ms = swing.expected_ms(
        b.f32("fighter.progress"),
        b.f32("keys.last"),
        b.f32("fighter.base"),
    );
    Out::default().bytes(&ms.to_le_bytes()).0
}

/// Every registered port, by name.
pub fn ports() -> &'static [Port] {
    const PORTS: &[Port] = &[
        Port {
            name: "normalize_exact_in_place",
            run: |b| in_place3(b, em::normalize_exact),
        },
        // glam's normalise, which the exact normaliser's callers' ports use.
        Port {
            name: "normalize_exact_in_place_glam",
            run: |b| in_place3(b, Vec3::normalize),
        },
        Port {
            name: "normalize_approx_in_place",
            run: |b| in_place3(b, em::normalize_approx),
        },
        Port {
            name: "normalize_approx_into",
            run: |b| {
                let mut i = In(b);
                i.skip(12);
                Out::default().vec3(em::normalize_approx(i.vec3())).0
            },
        },
        // The ledger's port for the unmasked normaliser (the door leaf's
        // set-up): the masked one.
        Port {
            name: "normalize_approx_unmasked",
            run: |b| in_place3(b, em::normalize_approx),
        },
        Port {
            name: "normalize_quat_in_place",
            run: |b| {
                let v = In(b).vec4();
                let q = em::normalize_quat(Quat::from_xyzw(v.x, v.y, v.z, v.w));
                Out::default().f32s(&q.to_array()).0
            },
        },
        Port {
            name: "vec4_mul_4x3_difference",
            run: |b| {
                let mut i = In(b);
                i.skip(12);
                let v = i.vec4();
                let flat: [f32; 24] = i.array();
                let rows: [[f32; 3]; 8] =
                    std::array::from_fn(|r| [flat[3 * r], flat[3 * r + 1], flat[3 * r + 2]]);
                Out::default().vec3(em::vec4_mul_4x3_difference(v, &rows)).0
            },
        },
        Port {
            name: "triple_product",
            run: |b| {
                let mut i = In(b);
                let (a, c, d) = (i.vec4(), i.vec4(), i.vec4());
                Out::default()
                    .f32(em::triple_product(a.truncate(), c.truncate(), d.truncate()))
                    .0
            },
        },
        Port {
            name: "barycentric_exact",
            run: |b| {
                let mut i = In(b);
                i.skip(16);
                let p = i.vec3();
                let flat: [f32; 12] = i.array();
                let tet: [[f32; 3]; 4] =
                    std::array::from_fn(|r| [flat[3 * r], flat[3 * r + 1], flat[3 * r + 2]]);
                Out::default().f32s(&em::barycentric_exact(p, &tet)).0
            },
        },
        // The call sites' own form of the engine's lerp.
        Port {
            name: "vec3_lerp_into",
            run: |b| lerp3_into(b, |a, c, t| a + (c - a) * t),
        },
        // glam's lerp, which some ports call instead.
        Port {
            name: "vec3_lerp_into_glam",
            run: |b| lerp3_into(b, Vec3::lerp),
        },
        Port {
            name: "vec2_lerp_into",
            run: |b| {
                let mut i = In(b);
                i.skip(8);
                let (a, c, t) = (i.vec2(), i.vec2(), i.f32());
                Out::default().vec2(a + (c - a) * t).0
            },
        },
        Port {
            name: "min_components",
            run: |b| into3_2(b, em::min_components),
        },
        Port {
            name: "max_components",
            run: |b| into3_2(b, em::max_components),
        },
        Port {
            name: "min_component",
            run: |b| Out::default().f32(em::min_component(In(b).vec3())).0,
        },
        Port {
            name: "max_component",
            run: |b| Out::default().f32(em::max_component(In(b).vec3())).0,
        },
        Port {
            name: "largest_abs",
            run: |b| {
                Out::default()
                    .f32(exa_physics::sector::largest_abs(In(b).array()))
                    .0
            },
        },
        Port {
            name: "sign",
            run: |b| scalar(b, em::sign),
        },
        Port {
            name: "clamp",
            run: |b| {
                let mut i = In(b);
                let (a, lo, hi) = (i.f32(), i.f32(), i.f32());
                Out::default().f32(em::clamp(a, lo, hi)).0
            },
        },
        Port {
            name: "floor_to_i32",
            run: |b| Out::default().i32(em::floor_to_i32(In(b).f32())).0,
        },
        Port {
            name: "ceil_to_i32",
            run: |b| Out::default().i32(em::ceil_to_i32(In(b).f32())).0,
        },
        Port {
            name: "ln",
            run: |b| scalar(b, em::ln),
        },
        Port {
            name: "exp",
            run: |b| scalar(b, em::exp),
        },
        Port {
            name: "sin_cos",
            run: |b| {
                let (s, c) = em::sin_cos(In(b).f32());
                Out::default().f32(s).f32(c).0
            },
        },
        Port {
            name: "cosine",
            run: |b| {
                let mut i = In(b);
                let (a, c) = (i.vec3(), i.vec3());
                Out::default().f32(em::cosine(a, c)).0
            },
        },
        Port {
            name: "heading",
            run: |b| Out::default().f32(em::heading(In(b).vec3())).0,
        },
        Port {
            name: "direction_from_yaw",
            run: |b| Out::default().vec3(em::direction_from_yaw(In(b).f32())).0,
        },
        Port {
            name: "quaternion_from_axis_angle",
            run: |b| {
                let mut i = In(b);
                let (axis, angle) = (i.vec3(), i.f32());
                let q = em::quaternion_from_axis_angle(axis, angle);
                Out::default().f32s(&q.to_array()).0
            },
        },
        Port {
            name: "rgba8_lerp",
            run: rgba8_lerp,
        },
        Port {
            name: "face_new",
            run: face_new,
        },
        Port {
            name: "face_nearest",
            run: face_nearest,
        },
        Port {
            name: "door_open_span",
            run: door_open_span,
        },
        Port {
            name: "swing_expected_ms",
            run: swing_expected_ms,
        },
    ];
    PORTS
}
