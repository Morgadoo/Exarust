//! Ports registered by the second oracle lane (batch a): the engine's
//! vector and matrix helpers, quaternion and turn builders, projections.
//!
//! The original keeps a rigid frame as four rows of three (three axis rows
//! and the translation), a 4×4 as four rows of four, and multiplies row
//! vectors; a frame's rows are an `Affine3A`'s columns here.

use bevy::math::{Affine3A, Mat3A, Mat4, Quat, Vec3A, Vec4};
use exa_physics::engine_math as em;

#[allow(unused_imports)]
use super::{Case, In, Out, Port};

/// A rigid frame from its twelve stored floats.
fn frame(i: &mut In) -> Affine3A {
    let r: [f32; 12] = i.array();
    let row = |k: usize| Vec3A::new(r[3 * k], r[3 * k + 1], r[3 * k + 2]);
    Affine3A {
        matrix3: Mat3A::from_cols(row(0), row(1), row(2)),
        translation: row(3),
    }
}

/// A 4×4 from its sixteen stored floats (each stored row a column).
fn mat4(i: &mut In) -> Mat4 {
    Mat4::from_cols_array(&i.array())
}

/// A frame's twelve floats.
fn frame_out(f: &Affine3A) -> Vec<u8> {
    let m = &f.matrix3;
    Out::default()
        .f32s(&m.x_axis.to_array())
        .f32s(&m.y_axis.to_array())
        .f32s(&m.z_axis.to_array())
        .f32s(&f.translation.to_array())
        .0
}

/// A 3×3 turn's nine floats, row by row.
fn mat3_out(m: &Mat3A) -> Vec<u8> {
    Out::default()
        .f32s(&m.x_axis.to_array())
        .f32s(&m.y_axis.to_array())
        .f32s(&m.z_axis.to_array())
        .0
}

fn vec4(v: Vec4) -> Vec<u8> {
    Out::default().f32s(&v.to_array()).0
}

pub(super) const PORTS: &[Port] = &[
    Port {
        name: "vec4_dot_rows",
        run: |b| {
            let mut i = In(b);
            i.skip(16);
            let v = i.vec4();
            vec4(em::dot_rows4(v, &mat4(&mut i)))
        },
    },
    Port {
        name: "vec4_lerp_into",
        run: |b| {
            let mut i = In(b);
            i.skip(16);
            let (a, c, t) = (i.vec4(), i.vec4(), i.f32());
            vec4(em::lerp4(a, c, t))
        },
    },
    Port {
        name: "vec4_lerp_in_place",
        run: |b| {
            let mut i = In(b);
            let (a, c, t) = (i.vec4(), i.vec4(), i.f32());
            vec4(em::lerp4(a, c, t))
        },
    },
    Port {
        name: "vec4_negate_into",
        run: |b| {
            let mut i = In(b);
            i.skip(16);
            vec4(em::negate4(i.vec4()))
        },
    },
    Port {
        name: "vec3_negate_into",
        run: |b| {
            let mut i = In(b);
            i.skip(12);
            Out::default().vec3(em::negate3(i.vec3())).0
        },
    },
    Port {
        name: "vec3_negate_in_place",
        run: |b| Out::default().vec3(em::negate3(In(b).vec3())).0,
    },
    Port {
        name: "length_approx",
        run: |b| Out::default().f32(em::length_approx(In(b).vec3())).0,
    },
    Port {
        name: "distance_approx",
        run: |b| {
            let mut i = In(b);
            let (a, c) = (i.vec3(), i.vec3());
            Out::default().f32(em::distance_approx(a, c)).0
        },
    },
    Port {
        name: "normalize_exact_into",
        run: |b| {
            let mut i = In(b);
            i.skip(12);
            Out::default().vec3(em::normalize_exact(i.vec3())).0
        },
    },
    Port {
        name: "mat4_point",
        run: |b| {
            let mut i = In(b);
            i.skip(12);
            let p = i.vec3();
            Out::default().vec3(em::mat4_point(p, &mat4(&mut i))).0
        },
    },
    Port {
        name: "point_out_of_frame",
        run: |b| {
            let mut i = In(b);
            i.skip(12);
            let p = i.vec3();
            Out::default()
                .vec3(em::point_out_of_frame(p, &frame(&mut i)))
                .0
        },
    },
    Port {
        name: "point_into_frame",
        run: |b| {
            let mut i = In(b);
            i.skip(12);
            let p = i.vec3();
            Out::default()
                .vec3(em::point_into_frame(p, &frame(&mut i)))
                .0
        },
    },
    Port {
        name: "frame_product",
        run: |b| {
            let mut i = In(b);
            i.skip(48);
            let (a, c) = (frame(&mut i), frame(&mut i));
            frame_out(&em::frame_product(&a, &c))
        },
    },
    Port {
        name: "mat4_product",
        run: |b| {
            let mut i = In(b);
            i.skip(64);
            let (a, c) = (mat4(&mut i), mat4(&mut i));
            Out::default()
                .f32s(&em::mat4_product(&a, &c).to_cols_array())
                .0
        },
    },
    Port {
        name: "rigid_inverse",
        run: |b| {
            let mut i = In(b);
            i.skip(48);
            frame_out(&em::rigid_inverse(&frame(&mut i)))
        },
    },
    Port {
        name: "frame_into",
        run: |b| {
            let mut i = In(b);
            i.skip(48);
            let (a, c) = (frame(&mut i), frame(&mut i));
            frame_out(&em::frame_into(&a, &c))
        },
    },
    Port {
        name: "vec4_mul_4x3",
        run: |b| {
            let mut i = In(b);
            i.skip(12);
            let v = i.vec4();
            let flat: [f32; 12] = i.array();
            let rows: [[f32; 3]; 4] =
                std::array::from_fn(|r| [flat[3 * r], flat[3 * r + 1], flat[3 * r + 2]]);
            Out::default().vec3(em::vec4_mul_4x3(v, &rows)).0
        },
    },
    Port {
        name: "vec3_lerp_in_place",
        run: |b| {
            let mut i = In(b);
            let (a, c, t) = (i.vec3(), i.vec3(), i.f32());
            Out::default().vec3(em::lerp3(a, c, t)).0
        },
    },
    Port {
        name: "quaternion_product",
        run: |b| {
            let mut i = In(b);
            let (q1, q2) = (i.vec4(), i.vec4());
            let q = em::quaternion_product(Quat::from_vec4(q1), Quat::from_vec4(q2));
            Out::default().f32s(&q.to_array()).0
        },
    },
    Port {
        name: "scale_mat3",
        run: |b| {
            let mut i = In(b);
            let r: [f32; 9] = i.array();
            let s = i.f32();
            let mut m = Mat3A::from_cols_array(&r);
            em::scale(&mut m, s);
            mat3_out(&m)
        },
    },
    Port {
        name: "mat3_to_quaternion",
        run: |b| {
            let mut i = In(b);
            i.skip(16);
            let m = Mat3A::from_cols_array(&i.array());
            Out::default()
                .f32s(&em::mat3_to_quaternion(&m).to_array())
                .0
        },
    },
    Port {
        name: "barycentric_fast",
        run: |b| {
            let mut i = In(b);
            i.skip(16);
            let p = i.vec3();
            let flat: [f32; 12] = i.array();
            let tet: [[f32; 3]; 4] =
                std::array::from_fn(|r| [flat[3 * r], flat[3 * r + 1], flat[3 * r + 2]]);
            Out::default().f32s(&em::barycentric_fast(p, &tet)).0
        },
    },
    Port {
        name: "rotate_about",
        run: |b| {
            let mut i = In(b);
            i.skip(12);
            let (v, axis, angle) = (i.vec3(), i.vec3(), i.f32());
            Out::default().vec3(em::rotate_about(axis, v, angle)).0
        },
    },
    Port {
        name: "axis_angle_to_mat3",
        run: |b| {
            let mut i = In(b);
            i.skip(36);
            let (axis, angle) = (i.vec3(), i.f32());
            mat3_out(&em::axis_angle_to_mat3(axis, angle))
        },
    },
    Port {
        name: "from_euler",
        run: |b| {
            let mut i = In(b);
            i.skip(36);
            mat3_out(&em::from_euler(i.vec3()))
        },
    },
    Port {
        name: "frustum",
        run: |b| {
            let mut i = In(b);
            i.skip(64);
            let p: [f32; 6] = i.array();
            let m = em::frustum(p[0], p[1], p[2], p[3], p[4], p[5]);
            Out::default().f32s(&m.to_cols_array()).0
        },
    },
    Port {
        name: "perspective",
        run: |b| {
            let mut i = In(b);
            i.skip(64);
            let (fov, aspect) = (i.f32(), i.f32());
            let (near, far) = (i.f32(), i.f32());
            let offset = [i.f32(), i.f32()];
            let viewport = [i32::from_le_bytes(i.bytes()), i32::from_le_bytes(i.bytes())];
            let m = em::perspective(fov, aspect, near, far, offset, viewport);
            Out::default().f32s(&m.to_cols_array()).0
        },
    },
    // The engine's global generator, its seed carried in the case.
    Port {
        name: "roll_unit",
        run: |b| {
            let mut roll = exa_game::thaum::engine::Roll(u32::from_le_bytes(In(b).bytes()));
            let draw = roll.unit();
            Out::default().bytes(&roll.0.to_le_bytes()).f32(draw).0
        },
    },
    // The supersampling LOD bias, the settings' base carried in the case.
    Port {
        name: "lod_bias",
        run: |b| {
            let mut i = In(b);
            let level = i32::from_le_bytes(i.bytes());
            let base = i.f32();
            Out::default()
                .f32(exa_assets::texture::lod_bias(base, level))
                .0
        },
    },
    // The GUI alpha and fade: the globals written (the alpha's byte, the
    // alpha the draw code reads and the pair handed to a skin).
    Port {
        name: "skin_set_alpha",
        run: |b| {
            let mut i = In(b);
            let (alpha, fade) = (i.f32(), i.f32());
            let mut globals = exa_ui::skin::SkinGlobals::default();
            globals.set_alpha(alpha, fade);
            Out::default()
                .bytes(&[globals.alpha_byte, 0, 0, 0])
                .f32(globals.alpha)
                .f32(globals.handed.0)
                .f32(globals.handed.1)
                .0
        },
    },
    Port {
        name: "cache_shader_step",
        run: cache_shader_step,
    },
    // The divination card's shadow outline for the GUI unit: eight points
    // of three floats (the third untouched, zero).
    Port {
        name: "div_card_shadow_outline",
        run: |b| {
            let unit = u32_of(b.field("unit.unit")) as i32;
            let mut out = Out::default();
            for [x, y] in exa_ui::div_card::shadow_outline(unit) {
                out = out.f32(x).f32(y).f32(0.0);
            }
            out.0
        },
    },
];

fn u32_of(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// One frame of the shader-cache screen: the form's fields laid out at
/// their places, then the hooks the frame asks for (repaint; focus, which
/// the original makes two calls; close).
fn cache_shader_step(b: &Case) -> Vec<u8> {
    use exa_ui::modal_screens::{CacheShaderScreen, WarmScreenPhase};
    let phase = match u32_of(b.field("form.phase")) {
        0 => WarmScreenPhase::FadingIn,
        1 => WarmScreenPhase::Warming,
        _ => WarmScreenPhase::FadingOut,
    };
    let mut screen = CacheShaderScreen {
        phase,
        start: u32_of(b.field("form.start")),
        next: u32_of(b.field("form.next")) as i32,
        alpha: b.u8("form.alpha"),
    };
    let answer = b.u8("warm.answer") != 0;
    let step = screen.step(u32_of(b.field("clock.now")), |_| answer);
    let mut form = vec![0u8; 0x170];
    form[0x15B] = screen.alpha;
    form[0x15C..0x160].copy_from_slice(&screen.start.to_le_bytes());
    let phase = match screen.phase {
        WarmScreenPhase::FadingIn => 0i32,
        WarmScreenPhase::Warming => 1,
        WarmScreenPhase::FadingOut => 2,
    };
    form[0x164..0x168].copy_from_slice(&phase.to_le_bytes());
    form[0x168..0x16C].copy_from_slice(&screen.next.to_le_bytes());
    let calls = [
        u8::from(step.repaint),
        u8::from(step.focus),
        u8::from(step.focus),
        u8::from(step.close),
    ];
    Out::default().bytes(&form).bytes(&calls).0
}
