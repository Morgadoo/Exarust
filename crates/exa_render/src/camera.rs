//! The scene camera and the 3D view settings, as the original builds them.
//!
//! The camera node is a scene node of class 3 carrying a vertical field of
//! view (70° when built) and, scaled by its 25 000-unit base, its near and
//! far planes: 0.004 and 0.5 of the base, i.e. 100 and 12 500 engine units.
//! The engine counts in centimetres, so Exarust's metre-space projection
//! takes a hundredth of each. A camera's field of view is overridden by the
//! view that owns it (the play camera's zoom, the viewer's `--fov`).
//!
//! A 3D view (the game's, the icon renderer's, the item viewer's) carries
//! the camera node, a six-plane convex volume for clipping, exposure 1,
//! bloom strength 1 and post-processing on. Exarust's shaders never apply
//! Bevy exposure, so the 1 is the effective exposure.
//!
//! Rules: `docs/RENDERING.md`.

use bevy::prelude::*;

/// The camera node's class in the original's scene graph.
pub const CAMERA_NODE_CLASS: u32 = 3;
/// The original's base distance the near and far planes scale.
pub const PLANE_BASE: f32 = 25_000.0;
/// The near plane as a share of [`PLANE_BASE`] (0.004, as stored).
pub const NEAR_SHARE: f32 = 0.004;
/// The far plane as a share of [`PLANE_BASE`] (0.5).
pub const FAR_SHARE: f32 = 0.5;
/// The vertical field of view a camera node is built with, degrees.
pub const CAMERA_FOV_DEGREES: f32 = 70.0;
/// Centimetres to the metre.
const CM_PER_METRE: f32 = 100.0;

/// The camera node's settings: vertical field of view (degrees) and its
/// near and far planes in engine units (centimetres).
///
/// Exarust's cameras do not use these planes yet: the play camera's far
/// plane is set by the scene's visibility and fog, which have not been
/// traced back to this node, so the planes are held here as data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraNode {
    /// Vertical field of view, degrees.
    pub fov_degrees: f32,
    /// Near plane, engine units.
    pub near: f32,
    /// Far plane, engine units.
    pub far: f32,
}

impl Default for CameraNode {
    /// The original's constructor: 70°, and 25 000 × 0.004 and × 0.5.
    fn default() -> Self {
        Self::with_fov(CAMERA_FOV_DEGREES)
    }
}

impl CameraNode {
    /// A camera node with the original's planes and the given vertical
    /// field of view.
    pub fn with_fov(fov_degrees: f32) -> Self {
        Self {
            fov_degrees,
            near: PLANE_BASE * NEAR_SHARE,
            far: PLANE_BASE * FAR_SHARE,
        }
    }

    /// The near plane in metres.
    pub fn near_metres(&self) -> f32 {
        self.near / CM_PER_METRE
    }

    /// The far plane in metres.
    pub fn far_metres(&self) -> f32 {
        self.far / CM_PER_METRE
    }

    /// The vertical field of view in radians.
    pub fn fov_radians(&self) -> f32 {
        self.fov_degrees.to_radians()
    }

    /// Writes the node's field of view and planes into a perspective
    /// projection (Bevy's projection is vertical too).
    pub fn apply(&self, projection: &mut PerspectiveProjection) {
        self.apply_planes(projection);
        projection.fov = self.fov_radians();
    }

    /// Writes only the near and far planes into a perspective projection
    /// (a view that overrides the field of view each frame).
    pub fn apply_planes(&self, projection: &mut PerspectiveProjection) {
        projection.near = self.near_metres();
        projection.far = self.far_metres();
    }

    /// A perspective projection with these settings.
    pub fn projection(&self) -> PerspectiveProjection {
        let mut projection = PerspectiveProjection::default();
        self.apply(&mut projection);
        projection
    }
}

/// Where a 3D view places its camera when built, engine units: off to one
/// side, above and in front of the origin, looking at a target node at the
/// origin. The viewers that own a view set their own pose afterwards.
pub const VIEW_CAMERA_START: [f32; 3] = [-300.0, 200.0, 500.0];

/// A rendered 3D view (the game's, the icon renderer's, the item viewer's)
/// as its constructor leaves it: exposure 1, bloom strength 1, post on and
/// no scene camera slot. Exarust keeps one game camera; the icon and item
/// views are Bevy cameras of their own, and its shaders never apply Bevy
/// exposure, so the 1 is the effective exposure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewSettings {
    /// Exposure multiplier the original applies; 1.
    pub exposure: f32,
    /// Bloom strength; 1.
    pub bloom: f32,
    /// Post-processing on.
    pub post: bool,
    /// The slot of the scene camera list the view's camera is registered
    /// in: -1 until a scene binds it.
    pub scene_slot: i32,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            bloom: 1.0,
            post: true,
            scene_slot: -1,
        }
    }
}

impl ViewSettings {
    /// Whether a scene has bound the view's camera (the original's release
    /// drops the scene's camera count only for a bound view).
    pub fn binds_camera(&self) -> bool {
        self.scene_slot >= 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_camera_is_seventy_degrees_with_the_original_planes() {
        let c = CameraNode::default();
        assert_eq!(c.fov_degrees, 70.0);
        // 25 000 × 0.004 in single precision is 100.000 01, × 0.5 is exact.
        assert!((c.near - 100.0).abs() < 1e-4);
        assert_eq!(c.far, 12_500.0);
        assert!((c.near_metres() - 1.0).abs() < 1e-6);
        assert_eq!(c.far_metres(), 125.0);
        let p = c.projection();
        assert_eq!(p.fov, 70f32.to_radians());
        assert!((p.near - 1.0).abs() < 1e-6);
        assert_eq!(p.far, 125.0);
    }

    #[test]
    fn another_field_of_view_keeps_the_planes() {
        let c = CameraNode::with_fov(45.0);
        assert!((c.near - 100.0).abs() < 1e-4);
        assert_eq!(c.far, 12_500.0);
        let mut p = PerspectiveProjection::default();
        c.apply(&mut p);
        assert!((p.fov - std::f32::consts::FRAC_PI_4).abs() < 1e-6);
    }

    #[test]
    fn a_view_defaults_to_exposure_and_bloom_one_with_post_on() {
        let v = ViewSettings::default();
        assert_eq!(v.exposure, 1.0);
        assert_eq!(v.bloom, 1.0);
        assert!(v.post);
        assert_eq!(v.scene_slot, -1);
        assert!(!v.binds_camera());
        assert!(ViewSettings { scene_slot: 0, ..v }.binds_camera());
    }
}
