//! The engine helpers the effect simulations lean on, with the original's
//! operation order: the vector length and squared distance sum their
//! squares as the original's vector routines do, a normalised vector uses
//! the processor's approximate reciprocal square root (zero stays zero),
//! the sign is 0 for 0, and the runtime's two random draws come from its
//! MT19937 stream: a whole number below `n` is the high half of the 32-bit
//! draw times `n` (`n + 1` for a negative `n`), a unit draw is the draw
//! over 2³². The engine's own 32-bit generator ([`Roll`]) steps
//! `x·0x0808_8405 + 1`.

use bevy::math::Vec3;

use crate::arena::Mt19937;

/// `√((x² + z²) + y²)`, the original's length.
pub fn length(v: Vec3) -> f32 {
    ((v.x * v.x + v.z * v.z) + v.y * v.y).sqrt()
}

/// `(x² + y²) + z²` of `a − b`, the original's squared distance.
pub fn dist_sq(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    (d.x * d.x + d.y * d.y) + d.z * d.z
}

/// The original's normalisation: times the approximate reciprocal square
/// root of `y² + (x² + z²)`; a zero vector stays zero.
pub fn normalize(v: Vec3) -> Vec3 {
    let sum = v.y * v.y + (v.x * v.x + v.z * v.z);
    if sum == 0.0 {
        return Vec3::ZERO;
    }
    v * crate::thaum::reciprocal_sqrt(sum)
}

/// The original's `min(a, b)`: `b` when `b <= a`, else `a`. A NaN in
/// either place leaves `a`, where the library's `min` would answer the
/// other operand.
pub fn least(a: f32, b: f32) -> f32 {
    if b <= a { b } else { a }
}

/// The original's `max(a, b)`: `b` when `a <= b`, else `a`. A NaN in
/// either place leaves `a`, where the library's `max` would answer the
/// other operand.
pub fn greatest(a: f32, b: f32) -> f32 {
    if a <= b { b } else { a }
}

/// 0 for 0, else ±1 by the sign bit.
pub fn sign(x: f32) -> f32 {
    if x == 0.0 { 0.0 } else { 1.0f32.copysign(x) }
}

/// `a + (b − a)·t`.
pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// `(a.z·b.z + a.y·b.y) + a.x·b.x`, the dot the arcs' plane test sums.
pub fn dot_zyx(a: Vec3, b: Vec3) -> f32 {
    (a.z * b.z + a.y * b.y) + a.x * b.x
}

/// The engine's sine and cosine, the one implementation in
/// [`exa_physics::engine_math`] (NaN past the saturating reduction).
pub use exa_physics::engine_math::sin_cos;

/// A vector turned about a unit `axis` by `angle`, by the engine's
/// axis-angle matrix.
pub fn rotate_about(v: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let (s, c) = sin_cos(angle);
    let t = 1.0 - c;
    let (ax, ay, az) = (axis.x, axis.y, axis.z);
    Vec3::new(
        (ay * s + ax * az * t) * v.z
            + (ax * ay * t - az * s) * v.y
            + ((1.0 - ax * ax) * c + ax * ax) * v.x,
        (ay * az * t - ax * s) * v.z
            + ((1.0 - ay * ay) * c + ay * ay) * v.y
            + (az * s + ax * ay * t) * v.x,
        ((1.0 - az * az) * c + az * az) * v.z
            + (ax * s + ay * az * t) * v.y
            + (ax * az * t - ay * s) * v.x,
    )
}

/// The runtime's random draws.
pub trait Draws {
    /// A whole number below `n` (the runtime's `random(n)`).
    fn below(&mut self, n: i32) -> i32;
    /// A draw in [0, 1), in double precision (the runtime's `random`).
    fn unit(&mut self) -> f64;
}

impl Draws for Mt19937 {
    fn below(&mut self, n: i32) -> i32 {
        let n = if n < 0 { n + 1 } else { n };
        let product = (i64::from(self.next_u32())).wrapping_mul(i64::from(n));
        ((product as u64) >> 32) as u32 as i32
    }

    fn unit(&mut self) -> f64 {
        Mt19937::unit(self)
    }
}

/// Three unit draws less one half: (x, lift·y, z), each drawn in turn, as
/// the arcs' and the wisps' random directions take them.
pub fn jitter(draws: &mut impl Draws, lift: f32) -> Vec3 {
    let x = (draws.unit() - 0.5) as f32;
    let y = (f64::from(lift) * (draws.unit() - 0.5)) as f32;
    let z = (draws.unit() - 0.5) as f32;
    Vec3::new(x, y, z)
}

/// The engine's 32-bit generator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Roll(pub u32);

impl Roll {
    /// Steps the state and returns it.
    pub fn step(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.0
    }

    /// A draw in [0, 1): the state over 2³².
    pub fn unit(&mut self) -> f32 {
        (f64::from(self.step()) * 2f64.powi(-32)) as f32
    }

    /// A whole percentage: the state times 100, high half.
    pub fn percent(&mut self) -> i32 {
        ((u64::from(self.step()) * 100) >> 32) as i32
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::Draws;

    /// Draws taken from fixed lists, for tests.
    #[derive(Debug, Clone, Default)]
    pub struct Fixed {
        pub units: Vec<f64>,
        pub wholes: Vec<i32>,
        pub unit_at: usize,
        pub whole_at: usize,
    }

    impl Fixed {
        pub fn new(units: &[f64], wholes: &[i32]) -> Self {
            Self {
                units: units.to_vec(),
                wholes: wholes.to_vec(),
                ..Self::default()
            }
        }
    }

    impl Draws for Fixed {
        fn below(&mut self, n: i32) -> i32 {
            let v = self
                .wholes
                .get(self.whole_at % self.wholes.len().max(1))
                .copied()
                .unwrap_or(1);
            self.whole_at += 1;
            v.min(n.max(1) - 1).max(0)
        }

        fn unit(&mut self) -> f64 {
            let v = self
                .units
                .get(self.unit_at % self.units.len().max(1))
                .copied()
                .unwrap_or(0.5);
            self.unit_at += 1;
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_sums_in_its_order() {
        let v = Vec3::new(3.0, 4.0, 12.0);
        assert_eq!(length(v), 13.0);
        assert_eq!(dist_sq(v, Vec3::ZERO), 169.0);
        assert_eq!(sign(-0.0), 0.0);
        assert_eq!(sign(-2.0), -1.0);
        assert_eq!(sign(0.5), 1.0);
        assert_eq!(normalize(Vec3::ZERO), Vec3::ZERO);
        assert!((normalize(v).length() - 1.0).abs() < 1e-3);
        assert_eq!(
            lerp(Vec3::ZERO, Vec3::new(2.0, 4.0, 6.0), 0.5),
            Vec3::new(1.0, 2.0, 3.0)
        );
        // The engine's min and max keep their first operand on a NaN.
        assert_eq!(least(2.0, 3.0), 2.0);
        assert_eq!(least(4.0, 3.0), 3.0);
        assert_eq!(greatest(2.0, 3.0), 3.0);
        assert_eq!(greatest(4.0, 3.0), 4.0);
        assert!(least(f32::NAN, 3.0).is_nan());
        assert_eq!(least(3.0, f32::NAN), 3.0);
        assert!(greatest(f32::NAN, 3.0).is_nan());
        assert_eq!(greatest(3.0, f32::NAN), 3.0);
    }

    #[test]
    fn the_engine_trig_tracks_the_library() {
        for k in -40..=40 {
            let a = k as f32 * 0.37;
            let (s, c) = sin_cos(a);
            assert!((s - a.sin()).abs() < 2e-6, "sin {a}");
            assert!((c - a.cos()).abs() < 2e-6, "cos {a}");
        }
        assert_eq!(sin_cos(0.0), (0.0, 1.0));
        let turned = rotate_about(Vec3::Z, Vec3::X, std::f32::consts::FRAC_PI_2);
        assert!((turned - Vec3::NEG_Y).length() < 1e-6);
    }

    #[test]
    fn the_runtime_draws_scale_the_stream() {
        let mut a = Mt19937::new(5489);
        let mut b = Mt19937::new(5489);
        let first = b.next_u32();
        assert_eq!(a.below(10), ((u64::from(first) * 10) >> 32) as i32);
        let second = b.next_u32();
        assert_eq!(a.unit(), f64::from(second) * 2f64.powi(-32));
    }

    #[test]
    fn the_engine_roll_steps_its_state() {
        let mut roll = Roll(0);
        assert_eq!(roll.step(), 1);
        assert_eq!(roll.step(), 0x0808_8406);
        let mut p = Roll(0);
        assert_eq!(p.percent(), 0);
        // u32::MAX steps to 2³² − 0x0808_8404 = 0xF7F7_7BFC, 96.86 % of 2³².
        assert_eq!(Roll(u32::MAX).percent(), 96);
    }
}
