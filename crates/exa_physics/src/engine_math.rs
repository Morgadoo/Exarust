//! The engine's maths unit: the processor's conversions, the fast
//! polynomial logarithm, exponential and trigonometry, angle wrapping, and
//! the vector, quaternion, matrix and projection helpers built on them.
//!
//! The arithmetic follows the original's instructions, not the decompiler's
//! tidied expressions: `MINSS`/`MAXSS` return their second operand when
//! either is NaN, the "round" helpers set the SIMD rounding mode (floor or
//! ceiling) before `CVTSS2SI`, integer conversions round ties to even, and
//! the float sums keep the original's association. Matrices are stored as
//! the three rows the original writes; a [`Mat3A`] holds them as its three
//! columns, which is the same rotation for column vectors.
//!
//! Evidence: the engine's fast-function and algebra routines in the
//! component map (Engine math: vectors, matrices, fast trig), with their
//! assembly for the constants and branch order.

// The original's compares deliberately send NaN down one branch; the
// negated compares say which.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Mat3A, Mat4, Quat, Vec3, Vec3A, Vec4};

/// The threshold below which an axis has no direction (single precision).
pub const AXIS_EPSILON: f32 = 1.192_089_98e-7;
/// The absolute roll cosine above which [`mat3_to_euler`] separates the
/// pitch from the yaw (double).
pub const GIMBAL_LIMIT: f64 = 0.005;
/// The fixed up vector the original's look-at basis starts from.
pub const UP: Vec3 = Vec3::Y;

/// The original's sign: the sign bit copied onto one, and zero for either
/// signed zero.
pub fn sign(x: f32) -> f32 {
    if x == 0.0 { 0.0 } else { 1.0_f32.copysign(x) }
}

/// `MINSS`: `a` when it is below `b` (ordered), else `b`; a NaN either side
/// gives `b`.
pub fn min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `MAXSS`: `a` when it is above `b` (ordered), else `b`; a NaN either side
/// gives `b`.
pub fn max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// `MAXSS` then `MINSS`; NaN in `lo` gives `lo`, NaN in `hi` gives `hi`.
pub fn clamp(a: f32, lo: f32, hi: f32) -> f32 {
    min(max(a, lo), hi)
}

/// `CVTSS2SI` with the rounding mode set downward: the floor, and the
/// processor's integer indefinite (`i32::MIN`) for a NaN or out-of-range
/// value.
pub fn floor_to_i32(x: f32) -> i32 {
    let rounded = x.floor();
    if rounded.is_nan() || rounded >= 2_147_483_648.0 || rounded < -2_147_483_648.0 {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// `CVTSS2SI` to a 64-bit word: ties to even, and `i64::MIN` for NaN or an
/// out-of-range value.
pub fn round_to_i64(x: f32) -> i64 {
    let rounded = x.round_ties_even();
    if rounded.is_nan()
        || rounded >= 9_223_372_036_854_775_808.0
        || rounded < -9_223_372_036_854_775_808.0
    {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// The processor's approximate reciprocal square root (`RSQRTSS`).
pub fn reciprocal_sqrt(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value.sqrt()
    }
}

/// The same with the rounding mode set upward: the ceiling. NaN and
/// out-of-range values give `i32::MIN`.
pub fn ceil_to_i32(x: f32) -> i32 {
    let rounded = x.ceil();
    if rounded.is_nan() || rounded >= 2_147_483_648.0 || rounded < -2_147_483_648.0 {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// `CVTPS2DQ` of one lane: ties to even, and `i32::MIN` for NaN or an
/// out-of-range value.
pub fn round_to_i32(x: f32) -> i32 {
    let rounded = x.round_ties_even();
    if rounded.is_nan() || rounded >= 2_147_483_648.0 || rounded < -2_147_483_648.0 {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// The fast natural logarithm: the exponent bits, the mantissa scaled to
/// one, a cubic on its square, and a constant per sign. A negative input
/// gives minus infinity, zero or a NaN the bit pattern of zero read the
/// mantissa way.
pub fn ln(x: f32) -> f32 {
    let bits = x.to_bits();
    let exponent = ((bits >> 23) as i32) as f32;
    // `CMPSS` predicate 6: not less than or equal, true for a NaN.
    let sign_term = if !(x <= 0.0) {
        f32::from_bits(0xC2B3_DE55)
    } else {
        f32::NEG_INFINITY
    };
    let m = f32::from_bits(bits & 0x007F_FFFF | 0x3F80_0000);
    let m2 = m * m;
    let polynomial = ((m * f32::from_bits(0x3CCC_A7FC) - f32::from_bits(0x3E79_8F62)) * m2
        + (m - f32::from_bits(0x4011_9122)))
        * m2
        + exponent * f32::from_bits(0x3F31_7218);
    (m * f32::from_bits(0x4059_7523) + sign_term) + polynomial
}

/// The fast exponential: scale by log2(e), clamp, split into integer and
/// mantissa, and a quartic on the mantissa read as a float with the integer
/// exponent's bits. Negative-of-large gives zero; NaN follows the clamp.
pub fn exp(x: f32) -> f32 {
    let top = f32::from_bits(0x4EFF_0000);
    let scaled = x * f32::from_bits(0x4B38_AA3B) + f32::from_bits(0x4E7E_0000);
    let clamped = if scaled < top { scaled } else { top };
    let mut integer = round_to_i32(clamped);
    if integer <= 0 {
        integer = 0;
    }
    let bits = integer as u32;
    let m = f32::from_bits(bits & 0x007F_FFFF | 0x3F80_0000);
    let mut polynomial = m * f32::from_bits(0x3C5D_BE77) + f32::from_bits(0xBB0B_4858);
    polynomial = polynomial * m + f32::from_bits(0x3E2A_9CBE);
    polynomial = polynomial * m + f32::from_bits(0x3E9F_C08B);
    polynomial = polynomial * m + f32::from_bits(0x3F02_8D05);
    f32::from_bits(bits & 0x7F80_0000) * polynomial
}

/// The fast sine: the angle's sign kept aside, reduced by the nearest even
/// multiple of π/4, then a minimax sine or cosine on the reduced angle by
/// the quadrant. The pick is the original's mask arithmetic: the chosen
/// polynomial plus the other one masked to `+0`.
pub fn sin(x: f32) -> f32 {
    let sign_bit = x.to_bits() & 0x8000_0000;
    let x = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    let j = round_to_i32(x * 1.273_239_5).wrapping_add(1) & !1;
    let reduced = x - j as f32 * 0.785_398_2;
    let z = reduced * reduced;
    let (sin_poly, cos_poly) = polynomials(z, reduced);
    let mask = quadrant_mask(j);
    let value =
        f32::from_bits(sin_poly.to_bits() & mask) + f32::from_bits(cos_poly.to_bits() & !mask);
    f32::from_bits(value.to_bits() ^ sign_bit ^ (((j & 4) as u32) << 29))
}

/// The engine's sine and cosine of `x` together. The first element is the
/// sine, the second the cosine.
///
/// Both come out of the original's mask arithmetic: the sine is the picked
/// polynomial plus the other masked to `+0`, the cosine is each polynomial
/// less its masked part, summed. So the cosine is NaN whenever either
/// polynomial is not finite, which happens for `|x|` from about 1.69e9 up,
/// where the reduction saturates (the sine's polynomial overflows to −∞).
pub fn sin_cos(x: f32) -> (f32, f32) {
    let sign_sin = x.to_bits() & 0x8000_0000;
    let x = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    let j = round_to_i32(x * 1.273_239_5).wrapping_add(1) & !1;
    let sign_sin = sign_sin ^ (((j & 4) as u32) << 29);
    let sign_cos = (!(j.wrapping_sub(2)) & 4) as u32;
    let reduced = x - j as f32 * 0.785_398_2;
    let z = reduced * reduced;
    let (sin_poly, cos_poly) = polynomials(z, reduced);
    let mask = quadrant_mask(j);
    let sin_part = f32::from_bits(sin_poly.to_bits() & mask);
    let cos_part = f32::from_bits(cos_poly.to_bits() & !mask);
    let sin = cos_part + sin_part;
    let cos = (sin_poly - sin_part) + (cos_poly - cos_part);
    (
        f32::from_bits(sin.to_bits() ^ sign_sin),
        f32::from_bits(cos.to_bits() ^ (sign_cos << 29)),
    )
}

/// All ones when the quadrant takes the sine polynomial (`j & 2` clear),
/// else zero: the original's integer compare mask.
fn quadrant_mask(j: i32) -> u32 {
    if j & 2 == 0 { u32::MAX } else { 0 }
}

/// The two minimax polynomials on the squared reduced angle `z`.
fn polynomials(z: f32, x: f32) -> (f32, f32) {
    let mut c = 2.443_315_7e-5 * z + -1.388_731_6e-3;
    c = c * z + 4.166_664_6e-2;
    c *= z;
    c *= z;
    c -= 0.5 * z;
    c += 1.0;
    let mut s = -1.951_529_6e-4 * z + 8.332_161e-3;
    s = s * z + -1.666_665_5e-1;
    s *= z;
    s *= x;
    s += x;
    (s, c)
}

/// The fast arctangent on `[−1, 1]`: `x·π/4 − (|x| − 1)(|x|·0.0663 +
/// 0.2447)·x`.
pub fn atan(x: f32) -> f32 {
    let a = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    x * 0.785_398_2 - (a - 1.0) * (a * 0.066_3 + 0.244_7) * x
}

const PI: f32 = std::f32::consts::PI;
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;

/// The fast two-argument arctangent, by octant on [`atan`]. The first
/// argument is the sine-like coordinate and the second the cosine-like one,
/// so `atan2(a, b)` is `a.atan2(b)` in Rust's terms. The compares send NaN
/// down the negative-second path, as the original's `COMISS` does.
pub fn atan2(a: f32, b: f32) -> f32 {
    if b.is_nan() || b < 0.0 {
        if a.is_nan() || a < 0.0 {
            if !(-b > -a) {
                -HALF_PI - atan(b / a)
            } else {
                atan(a / b) - PI
            }
        } else if !(a < -b) {
            HALF_PI - atan(b / a)
        } else {
            atan(a / b) + PI
        }
    } else if a.is_nan() || a < 0.0 {
        if b <= -a {
            -HALF_PI - atan(b / a)
        } else {
            atan(a / b)
        }
    } else if b <= a {
        HALF_PI - atan(b / a)
    } else {
        atan(a / b)
    }
}

/// The fast arc sine: `π/2 − (π/2 − (0.2075 − 0.0535x)x)√(1−x)` above zero
/// (and for NaN), `((0.0535x + 0.2075)x + π/2)√(1+x) − π/2` below.
pub fn asin_approx(x: f32) -> f32 {
    if !(x < 0.0) {
        let a = HALF_PI - (0.207_5 - x * 0.053_5) * x;
        HALF_PI - a * (1.0 - x).sqrt()
    } else {
        ((0.053_5 * x + 0.207_5) * x + HALF_PI) * (1.0 + x).sqrt() - HALF_PI
    }
}

/// The fast arc cosine: `(π/2 − 0.2075x + 0.0535x²)√(1−x)` above zero (and
/// for NaN), and π less the reflected form in double precision below.
pub fn acos_approx(x: f32) -> f32 {
    if !(x < 0.0) {
        let a = HALF_PI - x * 0.207_5;
        (x * x * 0.053_5 + a) * (1.0 - x).sqrt()
    } else {
        let a = x * 0.207_5 + HALF_PI;
        let r = (x * x * 0.053_5 + a) * (1.0 + x).sqrt();
        (std::f64::consts::PI - f64::from(r)) as f32
    }
}

/// The arc sine under its usual engine name.
pub fn asin(x: f32) -> f32 {
    asin_approx(x)
}

/// The arc cosine under its usual engine name.
pub fn acos(x: f32) -> f32 {
    acos_approx(x)
}

/// The angles of a turn under the name the body code has always used.
pub fn euler_of(m: &Mat3A) -> Vec3 {
    mat3_to_euler(m)
}

/// The difference of two angles wrapped into `[−π, π]`: the compares are in
/// double precision, the correction is the single-precision `2π`, and NaN
/// passes through.
pub fn angle_difference(a: f32, b: f32) -> f32 {
    let d = a - b;
    let wide = f64::from(d);
    if wide < -std::f64::consts::PI {
        d + 6.283_185_5
    } else if wide > std::f64::consts::PI {
        d - 6.283_185_5
    } else {
        d
    }
}

/// The product of a four-vector with a 4×3 matrix stored as four rows of
/// three: `(v·column0, v·column1, v·column2)`, the original's row-vector
/// times a matrix. The dot sums keep its horizontal order.
pub fn vec4_mul_4x3(v: Vec4, rows: &[[f32; 3]; 4]) -> Vec3 {
    let mut out = Vec3::ZERO;
    for column in 0..3 {
        let low = v.x * rows[0][column] + v.y * rows[1][column];
        let high = v.z * rows[2][column] + v.w * rows[3][column];
        out[column] = low + high;
    }
    out
}

/// The difference of two 4×3 products: two matrices of four rows of three,
/// the second 48 bytes after the first. Each output column is the first
/// product less the second.
pub fn vec4_mul_4x3_difference(first: Vec4, second: Vec4, rows: &[[f32; 3]; 8]) -> Vec3 {
    let mut one = [[0.0f32; 3]; 4];
    let mut two = [[0.0f32; 3]; 4];
    one.copy_from_slice(&rows[..4]);
    two.copy_from_slice(&rows[4..]);
    let first = vec4_mul_4x3(first, &one);
    let second = vec4_mul_4x3(second, &two);
    Vec3::new(first.x - second.x, first.y - second.y, first.z - second.z)
}

/// The scalar triple product `a · (b × c)`, summed in the original's
/// horizontal order.
pub fn triple_product(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let cross = Vec3::new(
        b.y * c.z - b.z * c.y,
        b.z * c.x - b.x * c.z,
        b.x * c.y - b.y * c.x,
    );
    a.x * cross.x + a.y * cross.y + a.z * cross.z
}

/// The barycentric coordinates of `point` in a tetrahedron whose four
/// corners are `tet[0..4]`, by four triple products and the processor's
/// approximate reciprocal (the original's fast variant).
pub fn barycentric_fast(point: Vec3, tet: &[[f32; 3]; 4]) -> [f32; 4] {
    let a = Vec3::from_array(tet[0]);
    let b = Vec3::from_array(tet[1]);
    let c = Vec3::from_array(tet[2]);
    let d = Vec3::from_array(tet[3]);
    let t1 = triple_product(point - b, d - b, c - b);
    let t2 = triple_product(point - a, c - a, d - a);
    let t3 = triple_product(point - a, d - a, b - a);
    let t4 = triple_product(point - a, b - a, c - a);
    let denominator = triple_product(b - a, c - a, d - a);
    #[cfg(target_arch = "x86_64")]
    // SAFETY: SSE is part of the x86_64 baseline.
    let rcp = unsafe {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
        _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(denominator)))
    };
    #[cfg(not(target_arch = "x86_64"))]
    let rcp = 1.0 / denominator;
    [rcp * t1, rcp * t2, rcp * t3, rcp * t4]
}

/// The same in exact division.
pub fn barycentric_exact(point: Vec3, tet: &[[f32; 3]; 4]) -> [f32; 4] {
    let a = Vec3::from_array(tet[0]);
    let b = Vec3::from_array(tet[1]);
    let c = Vec3::from_array(tet[2]);
    let d = Vec3::from_array(tet[3]);
    let t1 = triple_product(point - b, d - b, c - b);
    let t2 = triple_product(point - a, c - a, d - a);
    let t3 = triple_product(point - a, d - a, b - a);
    let t4 = triple_product(point - a, b - a, c - a);
    let denominator = triple_product(b - a, c - a, d - a);
    let rcp = 1.0 / denominator;
    [rcp * t1, rcp * t2, rcp * t3, rcp * t4]
}

/// The quaternion product as the original computes it: the vector part is
/// `(q2 × q1) + q2·q1.w + q1·q2.w` in that addition order, and the scalar
/// part `q1.w·q2.w − q1·q2`.
pub fn quaternion_product(q1: Quat, q2: Quat) -> Quat {
    let cross = q2.xyz().cross(q1.xyz());
    let mut xyz = cross + q2.xyz() * q1.w;
    xyz += q1.xyz() * q2.w;
    let w = q1.w * q2.w - (q1.z * q2.z + q1.y * q2.y + q1.x * q2.x);
    Quat::from_xyzw(xyz.x, xyz.y, xyz.z, w)
}

/// The quaternion of `angle` about `axis` (not necessarily unit): the sine
/// half-angle over the axis length, the cosine half-angle. A zero-length
/// axis gives the zero quaternion.
pub fn quaternion_from_axis_angle(axis: Vec3, angle: f32) -> Quat {
    let (sin, cos) = sin_cos(angle * 0.5);
    let length_squared = axis.x * axis.x + axis.y * axis.y + axis.z * axis.z;
    if length_squared <= AXIS_EPSILON {
        Quat::from_xyzw(0.0, 0.0, 0.0, 0.0)
    } else {
        let scale = sin / length_squared.sqrt();
        Quat::from_xyzw(axis.x * scale, axis.y * scale, axis.z * scale, cos)
    }
}

/// Four three-vectors widened to four-vectors with `w = 1`.
pub fn widen4(vectors: [Vec3; 4]) -> [Vec4; 4] {
    vectors.map(|v| v.extend(1.0))
}

/// A 4×4 affine matrix from a 3×3 turn and a translation: the three rows
/// copied, `w = 0`, and the translation in the fourth column.
pub fn affine4(turn: &Mat3A, translation: Vec3) -> Mat4 {
    Mat4::from_cols(
        turn.x_axis.extend(0.0),
        turn.y_axis.extend(0.0),
        turn.z_axis.extend(0.0),
        translation.extend(1.0),
    )
}

/// The turn of a yaw and a pitch, as the original writes its three rows:
/// `[cos p, 0, −sin p]`, `[sin y·sin p, cos y, sin y·cos p]`,
/// `[cos y·sin p, −sin y, cos y·cos p]`.
pub fn basis_from_yaw_pitch(yaw: f32, pitch: f32) -> Mat3A {
    let (sin_y, cos_y) = sin_cos(yaw);
    let (sin_p, cos_p) = sin_cos(pitch);
    Mat3A::from_cols(
        Vec3A::new(cos_p, 0.0, -sin_p),
        Vec3A::new(sin_y * sin_p, cos_y, sin_y * cos_p),
        Vec3A::new(cos_y * sin_p, -sin_y, cos_y * cos_p),
    )
}

/// The turn about X: rows `[1,0,0]`, `[0,cos,sin]`, `[0,−sin,cos]`.
pub fn rotation_x(angle: f32) -> Mat3A {
    let (sin, cos) = sin_cos(angle);
    Mat3A::from_cols(
        Vec3A::new(1.0, 0.0, 0.0),
        Vec3A::new(0.0, cos, sin),
        Vec3A::new(0.0, -sin, cos),
    )
}

/// The 3×3 turn of a quaternion, from the original's nine expressions.
pub fn quaternion_to_mat3(q: Quat) -> Mat3A {
    let (x, y, z, w) = (q.x, q.y, q.z, q.w);
    Mat3A::from_cols(
        Vec3A::new(
            1.0 - (z * z + y * y) * 2.0,
            (z * w + x * y) * 2.0,
            (z * x - y * w) * 2.0,
        ),
        Vec3A::new(
            (x * y - z * w) * 2.0,
            1.0 - (x * x + z * z) * 2.0,
            (x * w + y * z) * 2.0,
        ),
        Vec3A::new(
            (y * w + z * x) * 2.0,
            (y * z - x * w) * 2.0,
            1.0 - (x * x + y * y) * 2.0,
        ),
    )
}

/// The quaternion of a 3×3 turn, by the largest diagonal. The rows are
/// read as the original stores them; a zero axis from a NaN matrix keeps
/// the original's divisions.
pub fn mat3_to_quaternion(m: &Mat3A) -> Quat {
    let rows = [
        m.x_axis.to_array(),
        m.y_axis.to_array(),
        m.z_axis.to_array(),
    ];
    let trace = rows[0][0] + rows[1][1] + rows[2][2];
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt();
        let w = s * 0.5;
        let inv = 0.5 / s;
        Quat::from_xyzw(
            (rows[1][2] - rows[2][1]) * inv,
            (rows[2][0] - rows[0][2]) * inv,
            (rows[0][1] - rows[1][0]) * inv,
            w,
        )
    } else {
        let mut largest = usize::from(rows[1][1] > rows[0][0] && rows[1][1] != rows[0][0]);
        if rows[largest][largest] < rows[2][2] {
            largest = 2;
        }
        const CYCLE: [usize; 3] = [1, 2, 0];
        let next = CYCLE[largest];
        let other = CYCLE[next];
        let s = (rows[largest][largest] - (rows[next][next] + rows[other][other]) + 1.0).sqrt();
        let mut q = [0.0f32; 4];
        q[largest] = s * 0.5;
        let inv = if s != 0.0 { 0.5 / s } else { 0.0 };
        q[3] = (rows[next][other] - rows[other][next]) * inv;
        q[next] = (rows[largest][next] + rows[next][largest]) * inv;
        q[other] = (rows[largest][other] + rows[other][largest]) * inv;
        Quat::from_xyzw(q[0], q[1], q[2], q[3])
    }
}

/// The turn of `angle` about `axis`, from the original's expressions.
pub fn axis_angle_to_mat3(axis: Vec3, angle: f32) -> Mat3A {
    let (sin, cos) = sin_cos(angle);
    let one_minus = 1.0 - cos;
    let (x, y, z) = (axis.x, axis.y, axis.z);
    Mat3A::from_cols(
        Vec3A::new(
            (1.0 - x * x) * cos + x * x,
            x * y * one_minus - z * sin,
            y * sin + x * z * one_minus,
        ),
        Vec3A::new(
            z * sin + x * y * one_minus,
            (1.0 - y * y) * cos + y * y,
            y * z * one_minus - x * sin,
        ),
        Vec3A::new(
            x * z * one_minus - y * sin,
            y * z * one_minus + x * sin,
            (1.0 - z * z) * cos + z * z,
        ),
    )
}

/// `v` rotated `angle` about the unit `axis`, in the original's sum order.
pub fn rotate_about(axis: Vec3, v: Vec3, angle: f32) -> Vec3 {
    let (sin, cos) = sin_cos(angle);
    let one_minus = 1.0 - cos;
    let (x, y, z) = (axis.x, axis.y, axis.z);
    Vec3::new(
        (x * y * one_minus - z * sin) * v.y
            + ((1.0 - x * x) * cos + x * x) * v.x
            + (y * sin + x * z * one_minus) * v.z,
        ((1.0 - y * y) * cos + y * y) * v.y
            + (x * y * one_minus + z * sin) * v.x
            + (y * z * one_minus - x * sin) * v.z,
        (x * z * one_minus - y * sin) * v.x
            + (y * z * one_minus + x * sin) * v.y
            + ((1.0 - z * z) * cos + z * z) * v.z,
    )
}

/// The orthonormal basis whose forward (last axis) is `forward`: the fixed
/// up vector made square to it (the original's component subtraction), the
/// side as their cross. [`Vec3A::normalize_or_zero`] stands in for the
/// original's reciprocal square root.
pub fn look_rotation(forward: Vec3) -> Mat3A {
    let z = Vec3A::from(forward);
    let y = perpendicular(Vec3A::Y, z).normalize_or_zero();
    let x = y.cross(z).normalize_or_zero();
    Mat3A::from_cols(x, y, z)
}

/// `a` less its component along `b`: the original's projection helper. The
/// dot sums x and z first, then y.
fn perpendicular(a: Vec3A, b: Vec3A) -> Vec3A {
    let dot = a.x * b.x + a.z * b.z + a.y * b.y;
    a - b * dot
}

/// Each row of a 3×3 turn scaled by the matching component of `scale`.
pub fn scale_rows(m: &mut Mat3A, scale: Vec3) {
    m.x_axis *= scale.x;
    m.y_axis *= scale.y;
    m.z_axis *= scale.z;
}

/// Every row of a 3×3 turn scaled by `scale`.
pub fn scale(m: &mut Mat3A, scale: f32) {
    m.x_axis *= scale;
    m.y_axis *= scale;
    m.z_axis *= scale;
}

/// The turn of the original's angles (x pitch, y yaw, z roll).
pub fn from_euler(e: Vec3) -> Mat3A {
    let (sx, cx) = e.x.sin_cos();
    let (sy, cy) = e.y.sin_cos();
    let (sz, cz) = e.z.sin_cos();
    let cxsz = cx * sz;
    let sxsz = sx * sz;
    Mat3A::from_cols(
        Vec3A::new(cz * cy, sz, -cz * sy),
        Vec3A::new(sx * sy + -cxsz * cy, cx * cz, sx * cy + cxsz * sy),
        Vec3A::new(cx * sy + sxsz * cy, -sx * cz, cx * cy + -sxsz * sy),
    )
}

/// The original's angles of a turn: the roll from the first row's y, then
/// the pitch and yaw over the rolled cosine when it is over 0.005
/// (double), else no pitch and the yaw from the last row's x and z. All in
/// double precision.
pub fn mat3_to_euler(m: &Mat3A) -> Vec3 {
    let rows = [
        m.x_axis.to_array(),
        m.y_axis.to_array(),
        m.z_axis.to_array(),
    ];
    let roll = f64::from(rows[0][1]).asin() as f32;
    let c = f64::from(roll).cos() as f32;
    if f64::from(c.abs()) > GIMBAL_LIMIT {
        let pitch = f64::from(-rows[2][1] / c).atan2(f64::from(rows[1][1] / c)) as f32;
        let yaw = f64::from(-rows[0][2] / c).atan2(f64::from(rows[0][0] / c)) as f32;
        Vec3::new(pitch, yaw, roll)
    } else {
        let yaw = f64::from(rows[2][0]).atan2(f64::from(rows[2][2])) as f32;
        Vec3::new(0.0, yaw, roll)
    }
}

/// The inverse of a 4×4 matrix by Gauss-Jordan elimination with partial
/// pivoting, starting from the identity. A singular matrix divides by zero
/// as the original does.
pub fn inverse4(matrix: &Mat4) -> Mat4 {
    let mut source = matrix.to_cols_array();
    let mut inverse = Mat4::IDENTITY.to_cols_array();
    for column in 0..4usize {
        let mut pivot = column;
        for row in column + 1..4 {
            if source[row * 4 + column].abs() > source[pivot * 4 + column].abs() {
                pivot = row;
            }
        }
        if pivot != column {
            for k in 0..4 {
                source.swap(column * 4 + k, pivot * 4 + k);
                inverse.swap(column * 4 + k, pivot * 4 + k);
            }
        }
        let divisor = source[column * 4 + column];
        for k in 0..4 {
            source[column * 4 + k] /= divisor;
            inverse[column * 4 + k] /= divisor;
        }
        for row in 0..4 {
            if row == column {
                continue;
            }
            let factor = source[row * 4 + column];
            for k in 0..4 {
                source[row * 4 + k] -= source[column * 4 + k] * factor;
                inverse[row * 4 + k] -= inverse[column * 4 + k] * factor;
            }
        }
    }
    Mat4::from_cols_array(&inverse)
}

/// An OpenGL frustum matrix from the six planes; a zero far plane makes
/// the far side infinite.
pub fn frustum(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let mut m = Mat4::IDENTITY.to_cols_array();
    m[0] = (near * 2.0) / (right - left);
    m[5] = (near * 2.0) / (top - bottom);
    m[8] = (right + left) / (right - left);
    m[9] = (top + bottom) / (top - bottom);
    if far == 0.0 {
        m[10] = -1.0;
        m[14] = near * -2.0;
    } else {
        m[10] = -(far + near) / (far - near);
        m[14] = -(far * 2.0 * near) / (far - near);
    }
    m[11] = -1.0;
    m[15] = 0.0;
    Mat4::from_cols_array(&m)
}

/// A perspective matrix from a vertical field of view in degrees, the
/// aspect, the near and far planes, a sub-pixel offset in window pixels and
/// the viewport's size in pixels. The tangent is the double cosine over the
/// engine's single-precision sine, as the original mixes them.
pub fn perspective(
    vertical_fov_degrees: f32,
    aspect: f32,
    near: f32,
    far: f32,
    offset: [f32; 2],
    viewport: [i32; 2],
) -> Mat4 {
    let half_angle = vertical_fov_degrees * 0.017_453_292 * 0.5;
    let sine = sin(half_angle);
    let cosine = f64::cos(f64::from(half_angle));
    let half_height = (f64::from(near) / (cosine / f64::from(sine))) as f32;
    let half_width = half_height * aspect;
    let jitter_x = -(half_width * 2.0 * offset[0]) / viewport[0] as f32;
    let jitter_y = -(half_height * 2.0 * offset[1]) / viewport[1] as f32;
    let left = -half_width + jitter_x;
    let right = half_width + jitter_x;
    let bottom = -half_height + jitter_y;
    let top = half_height + jitter_y;
    let mut m = Mat4::IDENTITY.to_cols_array();
    m[0] = (near * 2.0) / (right - left);
    m[8] = (right + left) / (right - left);
    m[5] = (near * 2.0) / (top - bottom);
    m[9] = (top + bottom) / (top - bottom);
    if far == 0.0 {
        m[10] = -1.0;
        m[14] = near * -2.0;
    } else {
        m[10] = -(far + near) / (far - near);
        m[14] = -(far * 2.0 * near) / (far - near);
    }
    m[11] = -1.0;
    m[15] = 0.0;
    Mat4::from_cols_array(&m)
}

/// A matrix mapping a pixel rectangle's size and the viewport's corners to
/// normalized device coordinates, as the renderer's pixel-rectangle helper
/// does. `rect` is the rectangle's width and height, `viewport` its two
/// corner pairs.
pub fn pixel_rect(rect: [i32; 2], viewport: [[i32; 2]; 2]) -> Mat4 {
    let mut m = Mat4::IDENTITY.to_cols_array();
    let width = (viewport[1][0] - viewport[0][0]) as f32;
    let height = (viewport[1][1] - viewport[0][1]) as f32;
    m[0] = rect[0] as f32 / width;
    m[5] = rect[1] as f32 / height;
    m[12] = f64::from((rect[0] - viewport[0][0]) - viewport[1][0]) as f32 / f64::from(width) as f32;
    m[13] =
        f64::from((rect[1] - viewport[1][1]) - viewport[0][1]) as f32 / f64::from(-height) as f32;
    Mat4::from_cols_array(&m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn sign_and_the_processor_compares() {
        assert_eq!(sign(2.0), 1.0);
        assert_eq!(sign(-2.0), -1.0);
        assert_eq!(sign(0.0), 0.0);
        assert_eq!(sign(-0.0), 0.0);
        assert_eq!(sign(f32::NAN), 1.0);
        assert_eq!(min(1.0, 2.0), 1.0);
        assert_eq!(min(f32::NAN, 2.0), 2.0);
        assert!(max(1.0, f32::NAN).is_nan());
        assert_eq!(clamp(5.0, 0.0, 3.0), 3.0);
        assert_eq!(clamp(-5.0, 0.0, 3.0), 0.0);
        assert_eq!(floor_to_i32(2.9), 2);
        assert_eq!(floor_to_i32(-2.1), -3);
        assert_eq!(ceil_to_i32(2.1), 3);
        assert_eq!(ceil_to_i32(-2.9), -2);
        assert_eq!(floor_to_i32(f32::NAN), i32::MIN);
        assert_eq!(round_to_i32(2.5), 2);
        assert_eq!(round_to_i32(3.5), 4);
    }

    #[test]
    fn the_fast_logarithm_and_exponential() {
        assert_eq!(ln(1.0).to_bits(), 0x3780_0000);
        assert_eq!(ln(0.5).to_bits(), 0xBF31_7100);
        assert_eq!(ln(0.0), f32::NEG_INFINITY);
        assert_eq!(exp(0.0).to_bits(), 0x3F80_0016);
        assert_eq!(exp(1.0).to_bits(), 0x402D_F826);
        assert_eq!(exp(-1000.0), 0.0);
        assert!((ln(3.0) - 3.0f32.ln()).abs() < 1e-3);
    }

    #[test]
    fn the_fast_trigonometry_and_its_wrapping() {
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(sin_cos(0.0), (0.0, 1.0));
        for i in -40..40 {
            let x = i as f32 * 0.37;
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() < 1e-5, "{x}");
            assert!((c - x.cos()).abs() < 1e-5, "{x}");
        }
        for x in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
            assert!((atan(x) - x.atan()).abs() < 3e-3, "{x}");
        }
        assert!(close(asin(0.5), 0.523_980_4));
        assert!(close(asin(-0.5), -0.523_980_4));
        assert_eq!(asin(0.0), 0.0);
        assert!(close(acos(0.5), 1.046_816));
        assert!(close(acos(-0.5), 2.094_776_6));
        assert_eq!(acos(0.0), std::f32::consts::FRAC_PI_2);
        assert!(close(acos(-1.0), std::f32::consts::PI));
        assert!(close(
            angle_difference(3.0, -3.0),
            3.0 - (-3.0) - std::f32::consts::TAU
        ));
        assert_eq!(angle_difference(0.5, 0.25), 0.25);
        let nan = angle_difference(f32::NAN, 0.0);
        assert!(nan.is_nan());
    }

    #[test]
    fn the_small_linear_algebra() {
        let rows = [
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            [7.0, 8.0, 9.0],
            [10.0, 11.0, 12.0],
        ];
        // v times the matrix: each output is a dot with a column.
        assert_eq!(
            vec4_mul_4x3(Vec4::new(1.0, 1.0, 1.0, 1.0), &rows),
            Vec3::new(22.0, 26.0, 30.0)
        );
        let mut pair = [[0.0; 3]; 8];
        pair[..4].copy_from_slice(&rows);
        pair[4..].copy_from_slice(&rows);
        assert_eq!(
            vec4_mul_4x3_difference(
                Vec4::new(1.0, 0.0, 0.0, 0.0),
                Vec4::new(1.0, 0.0, 0.0, 0.0),
                &pair
            ),
            Vec3::new(0.0, 0.0, 0.0)
        );
        assert_eq!(triple_product(Vec3::X, Vec3::Y, Vec3::Z), 1.0);
        assert!(close(
            triple_product(
                Vec3::new(1.0, 2.0, 3.0),
                Vec3::new(4.0, 5.0, 6.0),
                Vec3::new(7.0, 8.0, 9.0)
            ),
            0.0
        ));
        // A tetrahedron at the origin and its unit axes: the coordinates of
        // a corner are one on that corner.
        let tet = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let b = barycentric_exact(Vec3::new(0.0, 1.0, 0.0), &tet);
        assert_eq!(b[2], 1.0);
        assert_eq!(b[0], 0.0);
        let fast = barycentric_fast(Vec3::new(0.25, 0.25, 0.25), &tet);
        assert!((fast[0] - 0.25).abs() < 1e-3, "{fast:?}");
    }

    #[test]
    fn quaternions_and_turns() {
        let q = quaternion_from_axis_angle(Vec3::Y, 0.0);
        assert_eq!(q, Quat::from_xyzw(0.0, 0.0, 0.0, 1.0));
        let q = quaternion_from_axis_angle(Vec3::ZERO, 1.0);
        assert_eq!(q, Quat::from_xyzw(0.0, 0.0, 0.0, 0.0));
        // The original's product of the identity with a turn keeps it.
        let half = quaternion_from_axis_angle(Vec3::Y, 1.0);
        let product = quaternion_product(Quat::IDENTITY, half);
        assert!(close(product.w, half.w));
        assert!(close(product.y, half.y));
        // A turn about X by 90° sends the original's first row to X.
        let m = rotation_x(std::f32::consts::FRAC_PI_2);
        assert!(close(m.x_axis.x, 1.0));
        assert!(m.y_axis.y.abs() < 1e-5);
        assert!(close(m.z_axis.z, 0.0));
        // The quaternion turn and back.
        let q = Quat::from_axis_angle(Vec3::new(0.3, -0.5, 0.8).normalize(), 0.7);
        let back = mat3_to_quaternion(&quaternion_to_mat3(q)).normalize();
        let direct = quaternion_to_mat3(q);
        let again = quaternion_to_mat3(back);
        assert!(
            (direct.x_axis - again.x_axis).length() < 1e-4,
            "{direct:?} {back:?} {again:?}"
        );
        // Axis-angle and the row rotation agree.
        let axis = Vec3::new(0.2, 0.9, -0.3).normalize();
        let a = axis_angle_to_mat3(axis, 0.6);
        let r = rotate_about(axis, Vec3::new(1.0, 2.0, 3.0), 0.6);
        let by_matrix = Vec3::new(
            a.x_axis.dot(Vec3::new(1.0, 2.0, 3.0).into()),
            a.y_axis.dot(Vec3::new(1.0, 2.0, 3.0).into()),
            a.z_axis.dot(Vec3::new(1.0, 2.0, 3.0).into()),
        );
        assert!((r - by_matrix).length() < 1e-5, "{r:?} {by_matrix:?}");
        // The yaw+pitch rows follow the original's signs.
        let m = basis_from_yaw_pitch(0.0, 0.5);
        let (sin_p, cos_p) = sin_cos(0.5);
        assert!(close(m.x_axis.x, cos_p));
        assert!(close(m.x_axis.z, -sin_p));
        // The look rotation's forward is its last axis.
        let look = look_rotation(Vec3::new(0.0, 0.0, -1.0));
        assert!(close(look.z_axis.z, -1.0));
        assert!(close(look.y_axis.y, 1.0));
        // The euler angles round-trip a small turn.
        let e = Vec3::new(0.2, 0.3, 0.1);
        assert!((mat3_to_euler(&from_euler(e)) - e).length() < 1e-5);
        assert_eq!(scale_rows_scaled(), 6.0);
    }

    fn scale_rows_scaled() -> f32 {
        let mut m = Mat3A::IDENTITY;
        scale_rows(&mut m, Vec3::new(1.0, 2.0, 3.0));
        scale(&mut m, 1.0);
        m.x_axis.x + m.y_axis.y + m.z_axis.z
    }

    #[test]
    fn the_four_by_four_inverse_and_the_projections() {
        let m = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))
            * Mat4::from_rotation_y(0.4)
            * Mat4::from_scale(Vec3::new(1.0, 2.0, 4.0));
        let product = m * inverse4(&m);
        for (a, b) in product
            .to_cols_array()
            .iter()
            .zip(Mat4::IDENTITY.to_cols_array())
        {
            assert!((a - b).abs() < 1e-4);
        }
        let f = frustum(-1.0, 1.0, -2.0, 2.0, 1.0, 0.0);
        assert_eq!(f.w_axis.z, -2.0);
        assert_eq!(f.w_axis.w, 0.0);
        let f = frustum(-1.0, 1.0, -2.0, 2.0, 1.0, 11.0);
        assert_eq!(f.z_axis.z, -(11.0 + 1.0) / (11.0 - 1.0));
        let p = perspective(90.0, 1.0, 1.0, 0.0, [0.0, 0.0], [1920, 1080]);
        assert!(close(p.x_axis.x, 1.0));
        assert!(close(p.y_axis.y, 1.0));
        assert_eq!(p.w_axis.z, -2.0);
        // A one-pixel offset shifts the centre by 2/width of the half width.
        let shifted = perspective(90.0, 1.0, 1.0, 11.0, [10.0, -10.0], [1000, 2000]);
        assert!(close(shifted.z_axis.x, -10.0 / 500.0));
        assert!(close(shifted.z_axis.y, 10.0 / 1000.0));
        let rect = pixel_rect([100, 50], [[0, 0], [200, 100]]);
        assert_eq!(rect.x_axis.x, 0.5);
        assert_eq!(rect.y_axis.y, 0.5);
        assert!(close(rect.w_axis.x, (100.0 - 0.0 - 200.0) / 200.0));
        assert!(close(rect.w_axis.y, (50.0 - 100.0 - 0.0) / -100.0));
    }
}
