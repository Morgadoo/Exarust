//! A convex volume: a variable-length list of planes, each with an optional
//! clipped polygon. The original uses it for frusta, sun-shadow volume
//! clipping, scene visibility and voxel GI; its constructor starts with an
//! empty list (count −1) and its destructor frees each plane's polygon
//! unless the polygons are shared or owned by another object, then the
//! plane array.
//!
//! Exarust builds a [`Frustum`](bevy::camera::primitives::Frustum) for its
//! cameras and sun cascades, but that holds a fixed six planes and no
//! polygons. [`ConvexVolume`] keeps the original's variable list and its
//! ownership flags, for the clipping code that needs them (portal frusta);
//! the polygon clipping itself belongs to the intersection unit.
//!
//! Rules: `docs/RENDERING.md`.

use bevy::prelude::*;

/// One plane of the volume: its inward normal and offset (`n·x + d ≥ 0` is
/// inside), and the clipped polygon it may carry.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvexPlane {
    /// The inward normal and offset.
    pub plane: Vec4,
    /// The plane's clipped polygon, if it has one.
    pub polygon: Option<Vec<Vec3>>,
}

impl ConvexPlane {
    /// A plane from its normal and offset, without a polygon.
    pub fn new(normal: Vec3, offset: f32) -> Self {
        Self {
            plane: normal.extend(offset),
            polygon: None,
        }
    }
}

/// A variable-length convex volume with the original's ownership flags.
#[derive(Clone, Debug, Default)]
pub struct ConvexVolume {
    planes: Vec<ConvexPlane>,
    /// The plane array is shared with another volume: its polygons are not
    /// freed with it.
    pub shared: bool,
    /// A polygon buffer at the original's `+0x18` pointer is owned
    /// elsewhere; its polygons are not freed with the volume.
    pub borrowed: bool,
    /// The original's count field: −1 for a fresh volume, else the number
    /// of planes it holds.
    count: i32,
}

impl ConvexVolume {
    /// A fresh volume: an empty plane list and the constructor's count of
    /// −1.
    pub fn new() -> Self {
        Self {
            planes: Vec::new(),
            shared: false,
            borrowed: false,
            count: -1,
        }
    }

    /// The original's raw count: −1 until a plane is added.
    pub fn raw_count(&self) -> i32 {
        self.count
    }

    /// The planes, in order.
    pub fn planes(&self) -> &[ConvexPlane] {
        &self.planes
    }

    /// Adds a plane at the end. The count becomes the list's length.
    pub fn push(&mut self, plane: ConvexPlane) {
        self.planes.push(plane);
        self.count = self.planes.len() as i32;
    }

    /// Removes every plane (the count returns to −1 unless the list is
    /// borrowed, which this models as still owning the buffer's lifetime).
    pub fn clear(&mut self) {
        self.planes.clear();
        self.count = -1;
    }

    /// How many of the planes carry a clipped polygon.
    pub fn polygons(&self) -> usize {
        self.planes.iter().filter(|p| p.polygon.is_some()).count()
    }

    /// Whether a point lies inside every plane, by the original's
    /// half-space test (`n·x + d ≥ 0`). An empty volume holds nothing.
    pub fn contains(&self, point: Vec3) -> bool {
        !self.planes.is_empty()
            && self
                .planes
                .iter()
                .all(|p| p.plane.truncate().dot(point) + p.plane.w >= 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_volume_has_no_planes_and_a_negative_count() {
        let mut v = ConvexVolume::new();
        assert_eq!(v.raw_count(), -1);
        assert!(v.planes().is_empty());
        assert_eq!(v.polygons(), 0);
        assert!(!v.contains(Vec3::ZERO));
        v.push(ConvexPlane::new(Vec3::X, 0.0));
        assert_eq!(v.raw_count(), 1);
        v.push(ConvexPlane {
            plane: Vec4::new(0.0, 1.0, 0.0, 2.0),
            polygon: Some(vec![Vec3::ZERO, Vec3::X, Vec3::Y]),
        });
        assert_eq!(v.raw_count(), 2);
        assert_eq!(v.polygons(), 1);
        // Inside both: x ≥ 0 and y ≥ −2.
        assert!(v.contains(Vec3::new(1.0, 0.0, 0.0)));
        assert!(!v.contains(Vec3::new(-0.5, 0.0, 0.0)));
        assert!(!v.contains(Vec3::new(1.0, -3.0, 0.0)));
        v.clear();
        assert_eq!(v.raw_count(), -1);
        assert!(v.planes().is_empty());
    }

    #[test]
    fn ownership_flags_start_clear() {
        let v = ConvexVolume::new();
        assert!(!v.shared);
        assert!(!v.borrowed);
    }
}
