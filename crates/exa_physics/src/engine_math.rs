//! The engine's maths unit: the processor's conversions, the fast
//! polynomial logarithm, exponential and trigonometry, angle wrapping, and
//! the vector, quaternion, matrix and projection helpers built on them.
//!
//! The arithmetic follows the original's instructions, not the decompiler's
//! tidied expressions: `MINSS`/`MAXSS` return their second operand when
//! either is NaN, the "round" helpers set the SIMD rounding mode (floor or
//! ceiling) before `CVTSS2SI`, integer conversions round ties to even, and
//! the float sums keep the original's association. Matrices are stored as
//! the rows the original writes, one after another; a [`Mat3A`] or
//! [`Mat4`] holds them as its columns (the same memory layout), so glam's
//! `m * v` is the original's row vector times the matrix.
//!
//! Evidence: the engine's fast-function and algebra routines in the
//! component map (Engine math: vectors, matrices, fast trig), with their
//! assembly for the constants and branch order.

// The original's compares deliberately send NaN down one branch; the
// negated compares say which.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Mat3A, Mat4, Quat, Vec3, Vec3A, Vec4};

/// The squared length at or below which an axis has no direction (single
/// precision).
pub const AXIS_EPSILON: f32 = 1.192_089_98e-7;
/// The absolute roll cosine above which [`mat3_to_euler`] separates the
/// pitch from the yaw (double).
pub const GIMBAL_LIMIT: f64 = 0.005;
/// The fixed up vector the original's look-at basis starts from.
pub const UP: Vec3 = Vec3::Y;

/// The engine's cosine between two vectors, summed in the original's order
/// in single precision: 0 when the product of their squared lengths is at
/// most 1e-8 (or either is NaN there), otherwise the dot over the square
/// root of that product.
pub fn cosine(a: Vec3, b: Vec3) -> f32 {
    let la = (a.y * a.y + a.x * a.x) + a.z * a.z;
    let lb = (b.y * b.y + b.x * b.x) + b.z * b.z;
    let p = lb * la;
    if p > 1e-8 {
        ((a.y * b.y + a.x * b.x) + a.z * b.z) / p.sqrt()
    } else {
        0.0
    }
}

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

/// `MAXSS` then `MINSS`. Each returns its second operand on NaN, so a NaN
/// `a` gives `lo` clamped by `hi`, a NaN `lo` gives `hi`, and a NaN `hi`
/// gives NaN.
pub fn clamp(a: f32, lo: f32, hi: f32) -> f32 {
    min(max(a, lo), hi)
}

/// `CVTSS2SI` with the rounding control set to round down (the mode's
/// other bits kept, the caller's mode restored after): the floor, and the
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

/// The same with the rounding control set to round up (and restored
/// after): the ceiling. NaN and out-of-range values give `i32::MIN`.
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

/// The fast natural logarithm: the biased exponent bits (with the sign bit
/// above them), the mantissa scaled into `[1, 2)`, a polynomial in it, and
/// a constant term that is −∞ for zero or a negative input (so those give
/// −∞). A NaN takes the finite constant and gives a finite value.
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
/// exponent's bits. A large negative input gives zero; a NaN or a large
/// positive one clamps to the infinity's exponent and gives +∞.
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
/// so `atan2(a, b)` is `a.atan2(b)` in Rust's terms. Each `COMISS` sends an
/// unordered (NaN) compare down the branch its parity jump takes: a NaN
/// coordinate counts as negative, and a NaN octant compare takes the
/// `atan(b / a)` side.
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
        if !(b > -a) {
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

/// The squared XZ length below which [`heading`] answers zero: the same
/// 1e-8 constant as [`cosine`]'s guard.
pub const HEADING_EPSILON: f32 = f32::from_bits(0x322B_CC77);

/// The fractional part of `x` by the processor's truncating conversion:
/// `x − trunc(x)`, so the sign follows `x`. NaN and a value outside the
/// 32-bit range convert to the integer indefinite, which is subtracted as
/// −2 147 483 648.
pub fn frac(x: f32) -> f32 {
    let whole = if x.is_nan() || x >= 2_147_483_648.0 || x < -2_147_483_648.0 {
        i32::MIN
    } else {
        x.trunc() as i32
    };
    x - whole as f32
}

/// The signed heading of a direction in the XZ plane: the arc cosine of
/// `z / |xz|`, negated when `x` is negative. Zero when the squared XZ
/// length is below [`HEADING_EPSILON`]. A cosine at or below −1 answers π,
/// above 1 zero, and a NaN length runs the division and the fast arc cosine
/// (which gives NaN). The squared length sums z first, then x.
pub fn heading(v: Vec3) -> f32 {
    let length_squared = v.z * v.z + v.x * v.x;
    if length_squared < HEADING_EPSILON {
        return 0.0;
    }
    let cosine = v.z / length_squared.sqrt();
    let angle = if cosine <= -1.0 {
        PI
    } else if cosine > 1.0 {
        0.0
    } else {
        acos_approx(cosine)
    };
    if v.x < 0.0 {
        f32::from_bits(angle.to_bits() ^ 0x8000_0000)
    } else {
        angle
    }
}

/// The ground-plane direction at `yaw`: `(sin yaw, 0, cos yaw)`, so yaw 0
/// looks along +Z and a positive yaw turns toward +X.
pub fn direction_from_yaw(yaw: f32) -> Vec3 {
    let (sin, cos) = sin_cos(yaw);
    Vec3::new(sin, 0.0, cos)
}

/// The full direction at an elevation and a heading, in the original's
/// argument order (elevation first): `(cos p·sin y, −sin p, cos p·cos y)`.
/// Both angles use the engine's fast sine and cosine.
pub fn direction_from_pitch_yaw(pitch: f32, yaw: f32) -> Vec3 {
    let (sin_y, cos_y) = sin_cos(yaw);
    let (sin_p, cos_p) = sin_cos(pitch);
    Vec3::new(cos_p * sin_y, -sin_p, cos_p * cos_y)
}

/// The angles of a turn under the name the body code has always used.
pub fn euler_of(m: &Mat3A) -> Vec3 {
    mat3_to_euler(m)
}

/// The difference of two angles moved by one turn when it lies outside
/// `[−π, π]` (once only, so a difference beyond 3π stays outside): the
/// compares are in double precision against π, the correction is the
/// single-precision `2π`, and NaN passes through.
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

/// One four-vector times two 4×3 matrices (eight rows of three, the second
/// matrix right after the first), the first product less the second. Unlike
/// [`vec4_mul_4x3`], each product sums its four terms one after another:
/// `((v.x·r0 + v.y·r1) + v.z·r2) + v.w·r3`.
pub fn vec4_mul_4x3_difference(v: Vec4, rows: &[[f32; 3]; 8]) -> Vec3 {
    let mut out = Vec3::ZERO;
    for column in 0..3 {
        let first = ((rows[0][column] * v.x + rows[1][column] * v.y) + rows[2][column] * v.z)
            + rows[3][column] * v.w;
        let second = ((v.x * rows[4][column] + v.y * rows[5][column]) + v.z * rows[6][column])
            + v.w * rows[7][column];
        out[column] = first - second;
    }
    out
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
/// part `q1.w·q2.w − q1·q2` with the dot summed `z + (y + x)`.
pub fn quaternion_product(q1: Quat, q2: Quat) -> Quat {
    let cross = q2.xyz().cross(q1.xyz());
    let mut xyz = cross + q2.xyz() * q1.w;
    xyz += q1.xyz() * q2.w;
    let w = q1.w * q2.w - (q1.z * q2.z + (q1.y * q2.y + q1.x * q2.x));
    Quat::from_xyzw(xyz.x, xyz.y, xyz.z, w)
}

/// The quaternion of `angle` about `axis` (not necessarily unit): the sine
/// half-angle over the axis length, the cosine half-angle. An axis whose
/// squared length is at most [`AXIS_EPSILON`] (or NaN) gets a zero vector
/// part; the cosine half-angle is written either way.
pub fn quaternion_from_axis_angle(axis: Vec3, angle: f32) -> Quat {
    let (sin, cos) = sin_cos(angle * 0.5);
    let length_squared = axis.x * axis.x + axis.y * axis.y + axis.z * axis.z;
    if !(length_squared > AXIS_EPSILON) {
        Quat::from_xyzw(0.0, 0.0, 0.0, cos)
    } else {
        let scale = sin / length_squared.sqrt();
        Quat::from_xyzw(axis.x * scale, axis.y * scale, axis.z * scale, cos)
    }
}

/// The four rows of a 4×3 matrix widened to a 4×4's with `w = 1` each.
pub fn widen4(vectors: [Vec3; 4]) -> [Vec4; 4] {
    vectors.map(|v| v.extend(1.0))
}

/// A 4×4 affine matrix from a 4×3 one (a 3×3 turn and a translation row):
/// the three turn rows with `w = 0`, then the translation with `w = 1`.
pub fn affine4(turn: &Mat3A, translation: Vec3) -> Mat4 {
    Mat4::from_cols(
        turn.x_axis.extend(0.0),
        turn.y_axis.extend(0.0),
        turn.z_axis.extend(0.0),
        translation.extend(1.0),
    )
}

/// The turn about X by `pitch` composed with the turn about Y by `yaw`
/// (the engine's fast sine and cosine), as the original writes its three
/// rows: `[cos y, 0, −sin y]`, `[sin p·sin y, cos p, sin p·cos y]`,
/// `[cos p·sin y, −sin p, cos p·cos y]`.
pub fn basis_from_pitch_yaw(pitch: f32, yaw: f32) -> Mat3A {
    let (sin_p, cos_p) = sin_cos(pitch);
    let (sin_y, cos_y) = sin_cos(yaw);
    Mat3A::from_cols(
        Vec3A::new(cos_y, 0.0, -sin_y),
        Vec3A::new(sin_p * sin_y, cos_p, sin_p * cos_y),
        Vec3A::new(cos_p * sin_y, -sin_p, cos_p * cos_y),
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

/// The quaternion of a 3×3 turn: from the trace when it is above zero,
/// else from the largest diagonal (the second only when strictly above the
/// first, the third when the pick is strictly below it; NaN keeps the
/// pick). When that diagonal's root is zero the other three parts are
/// multiplied by the zero root instead of `0.5 / s`.
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
        let mut largest = usize::from(rows[1][1] > rows[0][0]);
        if rows[largest][largest] < rows[2][2] {
            largest = 2;
        }
        const CYCLE: [usize; 3] = [1, 2, 0];
        let next = CYCLE[largest];
        let other = CYCLE[next];
        let s = (rows[largest][largest] - (rows[next][next] + rows[other][other]) + 1.0).sqrt();
        let mut q = [0.0f32; 4];
        q[largest] = s * 0.5;
        // `COMISS` against zero skips the division only on equal, so a
        // zero root stays the multiplier and NaN divides.
        let inv = if s != 0.0 { 0.5 / s } else { s };
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

/// `v` rotated `angle` about the unit `axis` (the engine's fast sine and
/// cosine): each component is a row of [`axis_angle_to_mat3`] dotted with
/// `v`, summed `r2·v.z + (r1·v.y + r0·v.x)` in the row's own index order.
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

/// The basis whose forward (last axis) is `forward`: the fixed [`UP`]
/// less its component along `forward`, then the side as that up crossed
/// with the forward. The up and the side are each scaled by the
/// processor's approximate reciprocal square root of their squared length
/// ([`reciprocal_sqrt`], `RSQRTSS`), as the original's normalise does, so
/// they are unit only to about 12 bits; a forward along the up gives NaN.
pub fn look_rotation(forward: Vec3) -> Mat3A {
    let z = Vec3A::from(forward);
    let y = fast_normalize(perpendicular(Vec3A::from(UP), z));
    let x = fast_normalize(y.cross(z));
    Mat3A::from_cols(x, y, z)
}

/// The original's three-lane dot: the lane products folded high onto low,
/// `(x + z) + (y + w)` with the empty fourth lane `+0`.
fn dot3(a: Vec3A, b: Vec3A) -> f32 {
    let p = a * b;
    (p.x + p.z) + (p.y + 0.0)
}

/// `a` less its component along `b`: the original's projection helper.
fn perpendicular(a: Vec3A, b: Vec3A) -> Vec3A {
    a - b * dot3(a, b)
}

/// `v` times the approximate reciprocal square root of its squared length.
fn fast_normalize(v: Vec3A) -> Vec3A {
    v * reciprocal_sqrt(dot3(v, v))
}

/// Each row of a 3×3 turn multiplied component by component by `scale`
/// (every row's x by `scale.x`, and so on).
pub fn scale_rows(m: &mut Mat3A, scale: Vec3) {
    let scale = Vec3A::from(scale);
    m.x_axis *= scale;
    m.y_axis *= scale;
    m.z_axis *= scale;
}

/// Every row of a 3×3 turn scaled by `scale`.
pub fn scale(m: &mut Mat3A, scale: f32) {
    m.x_axis *= scale;
    m.y_axis *= scale;
    m.z_axis *= scale;
}

/// The turn of the original's angles (x pitch, y yaw, z roll). The sines
/// and cosines are the runtime library's ([`library_sin_cos`]), not the
/// engine's fast polynomials.
pub fn from_euler(e: Vec3) -> Mat3A {
    let (sx, cx) = library_sin_cos(e.x);
    let (sy, cy) = library_sin_cos(e.y);
    let (sz, cz) = library_sin_cos(e.z);
    let cxsz = cx * sz;
    let sxsz = sx * sz;
    Mat3A::from_cols(
        Vec3A::new(cz * cy, sz, -cz * sy),
        Vec3A::new(sx * sy + -cxsz * cy, cx * cz, sx * cy + cxsz * sy),
        Vec3A::new(cx * sy + sxsz * cy, -sx * cz, cx * cy + -sxsz * sy),
    )
}

/// The original's angles of a turn: the roll from the first row's y, then
/// the pitch and yaw over the rolled cosine when its magnitude is over
/// 0.005 (double; NaN takes the other side), else no pitch and the yaw from
/// the last row's x and z. The arc sine, cosine and two-argument arctangent
/// are the runtime library's, in double precision, each result rounded to
/// single ([`library_arcsin`], [`library_arctan2`]).
pub fn mat3_to_euler(m: &Mat3A) -> Vec3 {
    let rows = [
        m.x_axis.to_array(),
        m.y_axis.to_array(),
        m.z_axis.to_array(),
    ];
    let roll = library_arcsin(f64::from(rows[0][1])) as f32;
    let c = f64::from(roll).cos() as f32;
    if f64::from(c.abs()) > GIMBAL_LIMIT {
        let pitch = library_arctan2(f64::from(-rows[2][1] / c), f64::from(rows[1][1] / c)) as f32;
        let yaw = library_arctan2(f64::from(-rows[0][2] / c), f64::from(rows[0][0] / c)) as f32;
        Vec3::new(pitch, yaw, roll)
    } else {
        let yaw = library_arctan2(f64::from(rows[2][0]), f64::from(rows[2][2])) as f32;
        Vec3::new(0.0, yaw, roll)
    }
}

/// The runtime library's single-precision sine and cosine. The original
/// runs the x87 `FSINCOS` on the value and stores each result rounded to
/// single; here the double-precision functions are rounded to single,
/// which gives the same value unless a result lies within a double's error
/// of a single-precision rounding boundary. Beyond `|x| = 2^63`, where the
/// x87 instruction leaves its operand unreduced, this is not the original.
pub fn library_sin_cos(x: f32) -> (f32, f32) {
    let (sin, cos) = f64::from(x).sin_cos();
    (sin as f32, cos as f32)
}

/// The runtime library's two-argument arctangent of `y` over `x`, in
/// double. A zero `x` (either sign) gives 0, π/2 or −π/2 by `y` (a NaN `y`
/// leaves the original's result unset; NaN here); otherwise the arctangent
/// of `y / x`, plus π when `x` is negative, less 2π when that passes π. So
/// `(−0, −1)` gives +π where `f64::atan2` gives −π. The single-argument
/// arctangent is the platform's `atan`.
pub fn library_arctan2(y: f64, x: f64) -> f64 {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    let mut angle = if x == 0.0 {
        if y == 0.0 {
            0.0
        } else if y > 0.0 {
            FRAC_PI_2
        } else if y < 0.0 {
            -FRAC_PI_2
        } else {
            f64::NAN
        }
    } else {
        (y / x).atan()
    };
    if x < 0.0 {
        angle += PI;
    }
    if angle > PI {
        angle -= TAU;
    }
    angle
}

/// The runtime library's arc sine in double: [`library_arctan2`] of `x`
/// over `√((1 + x)(1 − x))`.
pub fn library_arcsin(x: f64) -> f64 {
    library_arctan2(x, ((1.0 + x) * (1.0 - x)).sqrt())
}

/// The inverse of a 4×4 matrix by Gauss-Jordan elimination, starting from
/// the identity and working on the four stored rows (a [`Mat4`]'s
/// columns). The pivot search compares each lower row's magnitude with the
/// diagonal's, not with the best so far, so the last row larger than the
/// diagonal wins (NaN never does). A singular matrix divides by zero as the
/// original does.
pub fn inverse4(matrix: &Mat4) -> Mat4 {
    let mut source = matrix.to_cols_array();
    let mut inverse = Mat4::IDENTITY.to_cols_array();
    for column in 0..4usize {
        let mut pivot = column;
        let diagonal = source[column * 4 + column].abs();
        for row in column + 1..4 {
            if diagonal < source[row * 4 + column].abs() {
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
/// corner pairs. The differences are taken in 64-bit integers and every
/// quotient in double, then rounded to single.
pub fn pixel_rect(rect: [i32; 2], viewport: [[i32; 2]; 2]) -> Mat4 {
    let [[x0, y0], [x1, y1]] = viewport.map(|corner| corner.map(i64::from));
    let [width, height] = rect.map(i64::from);
    let mut m = Mat4::IDENTITY.to_cols_array();
    m[0] = (width as f64 / (x1 - x0) as f64) as f32;
    m[5] = (height as f64 / (y1 - y0) as f64) as f32;
    m[12] = (((width - x0) - x1) as f64 / (x1 - x0) as f64) as f32;
    m[13] = (((height - y1) - y0) as f64 / (y0 - y1) as f64) as f32;
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
        assert_eq!(clamp(f32::NAN, 1.0, 3.0), 1.0);
        assert_eq!(clamp(2.0, f32::NAN, 3.0), 3.0);
        assert!(clamp(2.0, 1.0, f32::NAN).is_nan());
        assert_eq!(floor_to_i32(2.9), 2);
        assert_eq!(floor_to_i32(-2.1), -3);
        assert_eq!(ceil_to_i32(2.1), 3);
        assert_eq!(ceil_to_i32(-2.9), -2);
        assert_eq!(floor_to_i32(f32::NAN), i32::MIN);
        assert_eq!(round_to_i32(2.5), 2);
        assert_eq!(round_to_i32(3.5), 4);
        assert_eq!(round_to_i32(-0.5), 0);
        assert_eq!(round_to_i32(1e10), i32::MIN);
        assert_eq!(round_to_i64(2.5), 2);
        assert_eq!(round_to_i64(3.5), 4);
        assert_eq!(round_to_i64(-2.5), -2);
        assert_eq!(round_to_i64(f32::NAN), i64::MIN);
        assert_eq!(round_to_i64(f32::INFINITY), i64::MIN);
        assert_eq!(ceil_to_i32(f32::NAN), i32::MIN);
        assert_eq!(floor_to_i32(3e9), i32::MIN);
        assert_eq!(floor_to_i32(-2_147_483_648.5), i32::MIN);
    }

    #[test]
    fn the_fast_logarithm_and_exponential() {
        assert_eq!(ln(1.0).to_bits(), 0x3780_0000);
        assert_eq!(ln(0.5).to_bits(), 0xBF31_7100);
        assert_eq!(ln(0.0), f32::NEG_INFINITY);
        assert_eq!(exp(0.0).to_bits(), 0x3F80_0016);
        assert_eq!(exp(1.0).to_bits(), 0x402D_F826);
        assert_eq!(exp(-1000.0), 0.0);
        assert_eq!(exp(f32::NAN), f32::INFINITY);
        assert_eq!(exp(1000.0), f32::INFINITY);
        assert_eq!(ln(-1.0), f32::NEG_INFINITY);
        assert!(ln(f32::NAN).is_finite());
        assert!((ln(3.0) - 3.0f32.ln()).abs() < 1e-3);
    }

    #[test]
    fn the_fast_trigonometry_and_its_wrapping() {
        assert_eq!(sin(0.0), 0.0);
        // Past the reduction's range the sine polynomial overflows and the
        // paired cosine is NaN, as the original's mask arithmetic gives.
        assert_eq!(sin(2.0e9), f32::NEG_INFINITY);
        let (s, c) = sin_cos(2.0e9);
        assert_eq!(s, f32::NEG_INFINITY);
        assert!(c.is_nan());
        let (s, c) = sin_cos(-2.0e9);
        assert_eq!(s, f32::INFINITY);
        assert!(c.is_nan());
        assert!(sin_cos(f32::NAN).0.is_nan() && sin_cos(f32::INFINITY).1.is_nan());
        for i in -40..40 {
            let x = i as f32 * 0.37;
            assert_eq!(sin(x).to_bits(), sin_cos(x).0.to_bits(), "{x}");
        }
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
            vec4_mul_4x3_difference(Vec4::new(1.0, 1.0, 1.0, 1.0), &pair),
            Vec3::new(0.0, 0.0, 0.0)
        );
        // The second matrix the identity rows and a zero row: the
        // difference is v times the first less v's first three.
        pair[4..].copy_from_slice(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0; 3]]);
        assert_eq!(
            vec4_mul_4x3_difference(Vec4::new(1.0, 2.0, 3.0, 4.0), &pair),
            Vec3::new(70.0 - 1.0, 80.0 - 2.0, 90.0 - 3.0)
        );
        // The sums run one after another: ((1 + 2^24) + 1) − 2^24 loses
        // both ones to rounding and gives 0, where the pairwise
        // (1 + 2^24) + (1 − 2^24) would give 1.
        let mut sequential = [[0.0f32; 3]; 8];
        sequential[0] = [1.0; 3];
        sequential[1] = [16_777_216.0; 3];
        sequential[2] = [1.0; 3];
        sequential[3] = [-16_777_216.0; 3];
        assert_eq!(
            vec4_mul_4x3_difference(Vec4::ONE, &sequential),
            Vec3::splat(0.0)
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
        // A zero axis keeps the cosine half-angle.
        let q = quaternion_from_axis_angle(Vec3::ZERO, 1.0);
        assert_eq!(q, Quat::from_xyzw(0.0, 0.0, 0.0, sin_cos(0.5).1));
        let q = quaternion_from_axis_angle(Vec3::new(f32::NAN, 0.0, 0.0), 0.0);
        assert_eq!(q, Quat::from_xyzw(0.0, 0.0, 0.0, 1.0));
        // The product's scalar part sums z + (y + x): with the products
        // x = 1, y = 2^24, z = −2^24 that is −2^24 + (2^24 + 1) = 0 (the
        // one is lost to rounding), where (z + y) + x would give 1.
        let a = Quat::from_xyzw(1.0, 4096.0, 4096.0, 0.0);
        let b = Quat::from_xyzw(1.0, 4096.0, -4096.0, 0.0);
        assert_eq!(quaternion_product(a, b).w, 0.0);
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
        // The pitch+yaw rows follow the original's signs.
        let m = basis_from_pitch_yaw(0.25, 0.5);
        let (sin_p, cos_p) = sin_cos(0.25);
        let (sin_y, cos_y) = sin_cos(0.5);
        assert_eq!(m.x_axis, Vec3A::new(cos_y, 0.0, -sin_y));
        assert_eq!(m.y_axis, Vec3A::new(sin_p * sin_y, cos_p, sin_p * cos_y));
        assert_eq!(m.z_axis, Vec3A::new(cos_p * sin_y, -sin_p, cos_p * cos_y));
        // The look rotation's forward is its last axis; the up and side
        // carry the approximate reciprocal square root.
        let look = look_rotation(Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(look.z_axis, Vec3A::new(0.0, 0.0, -1.0));
        assert_eq!(look.y_axis, Vec3A::new(0.0, reciprocal_sqrt(1.0), 0.0));
        assert!((look.y_axis.y - 1.0).abs() < 1e-3);
        assert!((look.x_axis.x + 1.0).abs() < 1e-3);
        assert!(look_rotation(Vec3::Y).x_axis.x.is_nan());
        // The library arctangent's quadrant fix-up: −0 over −1 is +π, a
        // tiny negative over −1 rounds to +π too, and a zero x is ±π/2.
        assert_eq!(library_arctan2(-0.0, -1.0), std::f64::consts::PI);
        assert_eq!(library_arctan2(-1e-300, -1.0), std::f64::consts::PI);
        assert_eq!(library_arctan2(-2.0, -0.0), -std::f64::consts::FRAC_PI_2);
        assert_eq!(library_arctan2(0.0, -0.0), 0.0);
        assert!((library_arctan2(-1.0, -1.0) + 0.75 * std::f64::consts::PI).abs() < 1e-15);
        assert_eq!(library_arcsin(1.0), std::f64::consts::FRAC_PI_2);
        assert!(library_arcsin(1.5).is_nan());
        // A half turn of pitch reads back as +π (the library's quadrant
        // fix-up of −0 over −1), where `f64::atan2` would give −π.
        let flipped = mat3_to_euler(&Mat3A::from_cols(
            Vec3A::X,
            Vec3A::new(0.0, -1.0, 0.0),
            Vec3A::new(0.0, 0.0, -1.0),
        ));
        assert_eq!(flipped.x, std::f32::consts::PI);
        // The euler angles round-trip a small turn.
        let e = Vec3::new(0.2, 0.3, 0.1);
        assert!((mat3_to_euler(&from_euler(e)) - e).length() < 1e-5);
        assert_eq!(scale_rows_scaled(), 6.0);
    }

    fn scale_rows_scaled() -> f32 {
        let mut m = Mat3A::IDENTITY;
        scale_rows(&mut m, Vec3::new(1.0, 2.0, 3.0));
        scale(&mut m, 1.0);
        // Every row is scaled by component: the first row's x by 1, its
        // y by 2.
        let mut full = Mat3A::from_cols(Vec3A::ONE, Vec3A::ONE, Vec3A::ONE);
        scale_rows(&mut full, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(full.x_axis, Vec3A::new(1.0, 2.0, 3.0));
        assert_eq!(full.z_axis, Vec3A::new(1.0, 2.0, 3.0));
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
        // The pivot is the last row above the diagonal, not the largest:
        // with rows 2 and 3 both above row 0's 1 in the first column, row
        // 3 (the smaller) is swapped up. This takes that path; the product
        // is still the identity.
        let pivoting = Mat4::from_cols(
            Vec4::new(1.0, 2.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(8.0, 0.0, 1.0, 0.0),
            Vec4::new(3.0, 0.0, 0.0, 1.0),
        );
        let product = pivoting * inverse4(&pivoting);
        for (a, b) in product
            .to_cols_array()
            .iter()
            .zip(Mat4::IDENTITY.to_cols_array())
        {
            assert!((a - b).abs() < 1e-5, "{product:?}");
        }
        let rect = pixel_rect([100, 50], [[0, 0], [200, 100]]);
        assert_eq!(rect.x_axis.x, 0.5);
        assert_eq!(rect.y_axis.y, 0.5);
        assert_eq!(rect.w_axis.x, (100.0 - 0.0 - 200.0) / 200.0);
        assert_eq!(rect.w_axis.y, (50.0 - 100.0 - 0.0) / -100.0);
        // The quotient is rounded once from double: 1/3 of a 3-pixel
        // viewport, and corners whose difference overflows 32 bits.
        assert_eq!(
            pixel_rect([1, 1], [[0, 0], [3, 3]]).x_axis.x,
            (1.0f64 / 3.0) as f32
        );
        let wide = pixel_rect([0, 0], [[i32::MIN, 0], [i32::MAX, 1]]);
        assert_eq!(wide.x_axis.x, 0.0);
        // (0 − (−2^31)) − (2^31 − 1) = 1 over 2^32 − 1.
        assert_eq!(wide.w_axis.x, (1.0f64 / 4_294_967_295.0) as f32);
    }

    #[test]
    fn frac_truncates_toward_zero() {
        assert_eq!(frac(2.75), 0.75);
        assert_eq!(frac(-2.75), -0.75);
        assert_eq!(frac(0.0), 0.0);
        assert_eq!(frac(-0.0), -0.0);
        assert_eq!(frac(5.0), 0.0);
        // Out of the 32-bit range the conversion is the indefinite integer.
        assert_eq!(frac(3.0e9), 3.0e9 - (-2_147_483_648.0));
        assert!(frac(f32::NAN).is_nan());
    }

    #[test]
    fn heading_follows_the_xz_direction() {
        // Yaw 0 looks along +Z; a quarter turn toward +X.
        assert_eq!(heading(Vec3::Z), 0.0);
        let quarter = heading(Vec3::X);
        assert!((quarter - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        // Negative x flips the sign; z = -1 gives π.
        assert!((heading(Vec3::NEG_X) + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((heading(Vec3::NEG_Z) - std::f32::consts::PI).abs() < 1e-6);
        // The height is ignored.
        assert_eq!(heading(Vec3::new(0.0, 100.0, 5.0)), 0.0);
        // A direction shorter than the guard is zero, even with x negative.
        assert_eq!(heading(Vec3::new(-1.0e-5, 0.0, 1.0e-5)), 0.0);
        // A cosine at the -1 end is π; NaN stays NaN.
        assert_eq!(heading(Vec3::new(0.0, 0.0, -0.0)), 0.0);
        assert!(heading(Vec3::new(f32::NAN, 0.0, 0.0)).is_nan());
    }

    #[test]
    fn directions_come_from_the_fast_trig() {
        assert_eq!(direction_from_yaw(0.0), Vec3::new(0.0, 0.0, 1.0));
        // The fast sine at a quarter turn is 1 to within its polynomial.
        let quarter = direction_from_yaw(std::f32::consts::FRAC_PI_2);
        assert!((quarter.x - 1.0).abs() < 1e-6);
        assert!(quarter.z.abs() < 1e-6);
        // Positive pitch raises the direction: the Y term is −sin(pitch).
        let up = direction_from_pitch_yaw(0.5, 0.0);
        assert!((up.y + 0.5f32.sin()).abs() < 2e-6);
        assert!((up.z - 0.5f32.cos()).abs() < 2e-6);
        assert_eq!(up.x, 0.0);
        // Elevation first, heading second: a heading of a quarter turn
        // points along +X.
        let right = direction_from_pitch_yaw(0.0, std::f32::consts::FRAC_PI_2);
        assert!((right.x - 1.0).abs() < 1e-6);
        assert_eq!(right.y, -0.0);
    }
}
