//! The terrain normal map's tangent frame, as the original's lit terrain
//! program builds it (and `terrain.wesl` mirrors).
//!
//! With `s` the interpolated vertex normal, the first axis is +X projected
//! onto the surface (`s × (X × s)` in the original's terms), and the second
//! is the cross of that axis with `s`. On flat ground they are +X and +Z;
//! they are orthonormal on any slope, unlike a separately projected +Z. The
//! palette's normal-map green channel then runs along the second axis, while
//! its v coordinate runs along −Z (the composite is mirrored in Z), so the
//! relief reads inverted against the image's v axis, as the original's does.

use bevy::math::Vec3;

/// The frame `(t, b)` the palette normal's x and y are applied along.
pub fn tangent_frame(normal: Vec3) -> (Vec3, Vec3) {
    let x = normal.normalize_or(Vec3::Y);
    let t = (Vec3::X - x * x.x).normalize_or(Vec3::X);
    let b = t.cross(x);
    (t, b)
}

/// The shaded normal from a tangent-space normal map sample `xy` (already
/// scaled by the layer's strength): z = √(1 − min(|xy|², 0.999)), then the
/// frame applied and the vector normalised.
pub fn normal(normal: Vec3, xy: bevy::math::Vec2) -> Vec3 {
    let s = normal.normalize_or(Vec3::Y);
    let (t, b) = tangent_frame(s);
    let z = (1.0 - xy.length_squared().min(0.999)).max(0.0).sqrt();
    (t * xy.x + b * xy.y + s * z).normalize_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec2;

    #[test]
    fn flat_ground_uses_plus_x_and_plus_z() {
        let (t, b) = tangent_frame(Vec3::Y);
        assert_eq!(t, Vec3::X);
        assert_eq!(b, Vec3::Z);
        // Green tilts the normal toward +Z, red toward +X.
        assert!(normal(Vec3::Y, Vec2::new(0.5, 0.0)).x > 0.4);
        assert!(normal(Vec3::Y, Vec2::new(0.0, 0.5)).z > 0.4);
        assert_eq!(normal(Vec3::Y, Vec2::new(0.0, 0.0)), Vec3::Y);
    }

    #[test]
    fn the_frame_stays_orthonormal_on_a_slope() {
        // A ground normal tilted 30° toward +Z.
        let s = Vec3::new(0.0, 30f32.to_radians().cos(), 30f32.to_radians().sin());
        let (t, b) = tangent_frame(s);
        assert!((t.length() - 1.0).abs() < 1e-6);
        assert!((b.length() - 1.0).abs() < 1e-6);
        assert!(t.dot(s).abs() < 1e-6);
        assert!(b.dot(s).abs() < 1e-6);
        assert!(t.dot(b).abs() < 1e-6);
        assert!((t - Vec3::X).length() < 1e-6);
        // The bitangent is the cross, not +Z projected: it gains a −Y part.
        assert!(b.y < -0.4, "{b:?}");
        // A full tangent-space normal reconstructs the vertex normal.
        let n = normal(s, Vec2::ZERO);
        assert!((n - s).length() < 1e-6);
    }

    #[test]
    fn steep_samples_are_capped_before_the_square_root() {
        // |xy|² = 4 is capped at 0.999, so z stays finite and the vector is
        // normalised.
        let n = normal(Vec3::Y, Vec2::new(2.0, 0.0));
        assert!(n.is_finite());
        assert!((n.length() - 1.0).abs() < 1e-6);
    }
}
