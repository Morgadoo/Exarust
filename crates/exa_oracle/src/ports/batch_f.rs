//! Ports registered by the collision lane (batch f): the particle-matrix
//! leaf routines (the widened rest points, their 4×4 inverse and the basis
//! times the particles).

use bevy::math::{Mat4, Vec3};
use exa_physics::engine_math as em;

use super::{Case, In, Out, Port};

/// Four points of three floats.
fn points(i: &mut In) -> [[f32; 3]; 4] {
    let flat: [f32; 12] = i.array();
    std::array::from_fn(|r| [flat[3 * r], flat[3 * r + 1], flat[3 * r + 2]])
}

fn inverse4(b: &Case) -> Vec<u8> {
    let mut i = In(b);
    i.skip(64);
    let m = Mat4::from_cols_array(&i.array());
    Out::default().f32s(&em::inverse4(&m).to_cols_array()).0
}

fn widen4(b: &Case) -> Vec<u8> {
    let mut i = In(b);
    i.skip(64);
    let p = points(&mut i).map(Vec3::from_array);
    let rows = em::widen4(p);
    let flat: Vec<f32> = rows.iter().flat_map(|r| r.to_array()).collect();
    Out::default().f32s(&flat).0
}

/// The product's rows; `pad` adds the untouched fourth lane of each row.
fn basis_points(b: &Case, out_len: usize, pad: bool) -> Vec<u8> {
    let mut i = In(b);
    i.skip(out_len);
    let p = points(&mut i);
    let basis = Mat4::from_cols_array(&i.array());
    let rows = em::basis_points(&basis, &p);
    let mut out = Out::default();
    for row in rows {
        out = out.f32s(&row);
        if pad {
            out = out.f32(0.0);
        }
    }
    out.0
}

/// The batch's ports.
pub(super) const PORTS: &[Port] = &[
    Port {
        name: "inverse4",
        run: inverse4,
    },
    Port {
        name: "widen4",
        run: widen4,
    },
    Port {
        name: "basis_points4",
        run: |b| basis_points(b, 64, true),
    },
    Port {
        name: "basis_points3",
        run: |b| basis_points(b, 48, false),
    },
];
