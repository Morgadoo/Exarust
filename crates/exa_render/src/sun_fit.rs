//! The orthographic region of the original's single sun shadow map.
//!
//! The original keeps one orthographic depth map for the sun, not a cascade
//! chain. Its update has two modes. In a level with a terrain it fits the map
//! every frame: it copies the camera's view volume, cuts it by up to three
//! planes at the scene's bounds on the sides the camera looks towards, fits
//! the smallest sphere around the surviving corners ([`crate::bound_sphere`]),
//! converts that sphere to centimetres, quantises its radius to 200 cm steps
//! with a hysteresis, snaps the centre to the shadow map's texel grid and
//! gives the box a fixed ±8000 cm depth. The quantised radius carries over to the next frame, which is what
//! the hysteresis compares against. Every other level covers the whole scene
//! instead ([`SunFit::whole_scene`]): the scene box's centre, its radius as
//! half extent, the same radius as the up-sun reach and the scene's radius
//! down-sun, all raised to the minimum extent, and no texel snap.
//!
//! The maths here is the original's, in the original's order:
//!
//! - The planes are chosen by the sign of each component of the camera's
//!   backward axis (the third row of its frame): a negative component (the
//!   camera looks towards +axis) keeps `axis ≤ high`, a positive one
//!   `axis ≥ low`, a zero (or NaN) one adds no plane. On x and z, high and
//!   low are the scene box's centre ± half extent. On y they are the level's
//!   ceiling and floor when the level has an environment record, and
//!   otherwise the box's *x* centre ± half extent, as the original reads
//!   them ([`VerticalRange`]).
//! - Each face keeps the part where `normal · p − w ≤ 0`; the cut is the
//!   engine's convex-volume clip ([`exa_physics::intersect::ConvexVolume`]).
//! - The corners are merged within 0.1 cm² and scaled by 0.01 (the
//!   centimetre world into the metre units the sphere solver is tuned for),
//!   and the fitted centre and radius are multiplied by 100 again
//!   ([`volume_corners`], [`fitted_sphere`]).
//! - The radius becomes `200·round(r/200 + 0.4)`; the conversion is the
//!   processor's (nearest, ties to even; a NaN or an out-of-range value gives
//!   the lowest integer, [`convert_nearest`]). A step is taken only when the
//!   new value differs *and* the radius is more than 0.3 steps above (grow)
//!   or 0.1 steps below (shrink) the current half extent. The fitted mode
//!   has no minimum extent: the scene's minimum (1 000 cm in a level) is
//!   read only by the whole-scene mode.
//! - The centre is rotated into the light's basis and its x and y are
//!   snapped to the texel grid (`2·half_extent / map size`), again rounding
//!   to nearest. The grid is anchored at the light-space origin, as the
//!   original's is. The whole-scene mode does not snap.
//! - Fewer than two corners fit nothing: the original then leaves its map
//!   and matrices as they were, which [`SunFit::fit`] reports.
//!
//! Not reproduced here: the caster-side polygon offset and depth nudge of
//! the render routine are computed and returned ([`SunFit::caster_bias`])
//! but Bevy's shadow pass has no caster-side bias state of its own; the
//! softness the update writes into the sun's light is computed
//! ([`sun_softness`], [`whole_scene_softness`]) and can be read by a shader
//! filter, but Bevy's directional shadows have no filter-radius uniform
//! without its PCSS feature. The eight-tap `SdwSmp` upload itself is
//! [`crate::shader_bindings::SHADOW_TAPS`]. Rules: `docs/RENDERING.md` (Sun shadow fit).
//!
//! Evidence: the procedure listings and decompiler dump of the component, the
//! vector helpers they call (squared distance, normalise, negate, scale,
//! matrix point transform, the look-at basis), the camera view's fields
//! (its camera node, whose frame rows are its axes) and the level loader
//! that copies the environment record's floor and ceiling into the scene.

use bevy::math::{Mat4, UVec2, Vec3, Vec4};
use exa_physics::intersect::{ConvexVolume, Plane};

use crate::bound_sphere::{Sphere, bounding_sphere};

/// The step the fitted half extent is quantised to, cm (the original's
/// `200.0`).
pub const STEP_CM: f32 = 200.0;
/// The half extent before the first fit, cm: the shadow map's constructor
/// default, and the value the first hysteresis compares against.
pub const INITIAL_HALF_EXTENT_CM: f32 = 3000.0;
/// How far the box reaches on each side of its centre along the light, cm.
pub const DEPTH_CM: f32 = 8000.0;
/// The scene's minimum shadow half extent, cm: the scene's constructor
/// multiplies 1 000 by a global factor (1.0 in the shipped build). Only the
/// whole-scene mode reads it; the fitted mode has no minimum.
pub const MIN_HALF_EXTENT_CM: f32 = 1000.0;
/// A step up is taken only above this many steps (the original's `0.3`).
pub const GROW_STEPS: f64 = 0.3;
/// A step down is taken only below this many steps (the original's `0.1`).
pub const SHRINK_STEPS: f64 = 0.1;
/// Added to the radius in steps before rounding (the original's `0.4`).
pub const QUANTISE_BIAS: f64 = 0.4;
/// The world is in centimetres; the sphere solver is fed metres.
pub const POINT_SCALE: f32 = 0.01;
/// Corners this close (squared, cm²) are one corner (the original's `0.1`).
pub const CORNER_MERGE2: f32 = 0.1;
/// Past this |y| of the light's travel direction (compared in double
/// precision) the light's frame is the fixed one.
pub const FIXED_FRAME_Y: f64 = 0.9999;
/// The far plane of the original camera's constructor, cm (25 000 × 0.5);
/// its near plane is 100 cm. What the game later sets was not traced.
pub const CAMERA_FAR_CM: f32 = 12_500.0;
/// The softness the update writes when the computed one is below it (the
/// original's `0.15`).
pub const SOFTNESS_MIN: f32 = 0.15;
/// The double constant the softness is compared with and divided into (the
/// floor written is the single [`SOFTNESS_MIN`]).
pub const SOFTNESS_FLOOR: f64 = 0.15;
/// The softness is `(c + 1) · 400 / half extent` (the original's fixed 400 cm
/// divided by the fitted half extent).
pub const SOFTNESS_CM: f32 = 400.0;
/// Centimetres in a metre.
const CM_PER_METRE: f32 = 100.0;

/// A half-space the view volume is clipped by: a point is kept when
/// `normal · p − w ≤ 0`, and a NaN counts as outside, as the original's
/// `comiss` does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipPlane {
    pub normal: Vec3,
    pub w: f32,
}

impl ClipPlane {
    pub fn new(normal: Vec3, w: f32) -> Self {
        Self { normal, w }
    }

    /// Cuts `volume` by this plane, keeping the inner side.
    pub fn clip(&self, volume: &mut ConvexVolume) {
        // The volume's plane is `normal · p + offset`; −w gives the same
        // bits as the original's `normal · p − w`.
        volume.clip_by_plane(self.normal, -self.w);
    }
}

/// The scene's axis-aligned bounds as the fitted mode clips the view volume
/// by them, cm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneBox {
    pub centre: Vec3,
    /// Half extents.
    pub extent: Vec3,
}

impl SceneBox {
    /// The box of the world bounds `min..max`, cm.
    pub fn from_bounds(min: Vec3, max: Vec3) -> Self {
        Self {
            centre: (min + max) * 0.5,
            extent: (max - min) * 0.5,
        }
    }

    /// The high and low edges on `axis`: `centre + extent`, `centre −
    /// extent`.
    fn edges(&self, axis: usize) -> (f32, f32) {
        (
            self.centre[axis] + self.extent[axis],
            self.centre[axis] - self.extent[axis],
        )
    }
}

/// The vertical limits the fitted mode clips by, cm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerticalRange {
    pub high: f32,
    pub low: f32,
}

impl VerticalRange {
    /// The original's limits: the level's ceiling and floor (whole cm, as the
    /// environment record stores them) when the level has the record, and
    /// otherwise the scene box's x edges. The original reads the x centre
    /// and extent in that case, not the vertical ones; it also overwrites the
    /// box's vertical centre and extent with 150 and 650 cm first, which
    /// nothing in the fit reads.
    pub fn of_level(scene: &SceneBox, floor_and_ceiling_cm: Option<(f32, f32)>) -> Self {
        match floor_and_ceiling_cm {
            Some((floor, ceiling)) => Self {
                high: ceiling,
                low: floor,
            },
            None => {
                let (high, low) = scene.edges(0);
                Self { high, low }
            }
        }
    }
}

/// The fitted mode's clipping planes for a camera whose backward axis (its
/// frame's third row, opposite the view direction) is `view_back`: per axis,
/// a negative component keeps `axis ≤ high`, a positive one `axis ≥ low`,
/// and a zero or NaN one adds none. x and z take the scene box's edges, y
/// the [`VerticalRange`].
pub fn view_clip_planes(
    scene: &SceneBox,
    vertical: VerticalRange,
    view_back: Vec3,
) -> Vec<ClipPlane> {
    let mut planes = Vec::with_capacity(3);
    for axis in 0..3 {
        let (high, low) = if axis == 1 {
            (vertical.high, vertical.low)
        } else {
            scene.edges(axis)
        };
        let mut normal = Vec3::ZERO;
        normal[axis] = 1.0;
        if view_back[axis] < 0.0 {
            planes.push(ClipPlane::new(normal, high));
        }
        if view_back[axis] > 0.0 {
            planes.push(ClipPlane::new(-normal, -low));
        }
    }
    planes
}

/// The two vertical planes of the original's other camera mode (one beyond
/// the first two): `y ≤ high` and `y ≥ low`, from the same
/// [`VerticalRange`]. Exarust has no such camera mode yet.
pub fn slab_clip_planes(vertical: VerticalRange) -> [ClipPlane; 2] {
    [
        ClipPlane::new(Vec3::Y, vertical.high),
        ClipPlane::new(Vec3::NEG_Y, -vertical.low),
    ]
}

/// The light's orthonormal frame: `dir` is the light's travel direction (the
/// sun's rays point along it), `right` and `up` span the shadow map's plane.
/// Built as the original builds the sun's view matrix: `dir` normalised and
/// negated from the vector towards the sun, `right = dir × +Y`, `up =
/// right × dir`, and a fixed frame when the light points straight up or down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightBasis {
    pub right: Vec3,
    pub up: Vec3,
    pub dir: Vec3,
}

impl LightBasis {
    /// The frame of a light `towards_sun` (need not be unit length).
    pub fn from_towards_sun(towards_sun: Vec3) -> Self {
        // The original normalises, then negates.
        Self::frame(-normalised(towards_sun))
    }

    /// The frame of a unit travel direction.
    fn frame(dir: Vec3) -> Self {
        // |y| is widened and compared with a double 0.9999; a NaN takes the
        // regular path.
        if f64::from(dir.y.abs()) > FIXED_FRAME_Y {
            // The original's fixed frame (whichever way the light points):
            // right +X, up −Z, forward down.
            return Self {
                right: Vec3::X,
                up: Vec3::NEG_Z,
                dir: Vec3::NEG_Y,
            };
        }
        let right = normalised(dir.cross(Vec3::Y));
        let up = normalised(right.cross(dir));
        Self { right, up, dir }
    }

    /// `p` in the light's frame: `(right · p, up · p, dir · p)`, as the
    /// original rotates the fitted centre.
    pub fn to_light(&self, p: Vec3) -> Vec3 {
        Vec3::new(p.dot(self.right), p.dot(self.up), p.dot(self.dir))
    }

    /// The world point of a light-space point: `right·x + up·y + dir·z`.
    pub fn from_light(&self, p: Vec3) -> Vec3 {
        self.right * p.x + self.up * p.y + self.dir * p.z
    }
}

/// A vector scaled to unit length, summed `(x² + z²) + y²` and zero left
/// zero, as the original's normalise. The original uses the processor's
/// approximate reciprocal square root; this uses the exact one, so the
/// result differs in the last dozen bits.
fn normalised(v: Vec3) -> Vec3 {
    let squared = (v.x * v.x + v.z * v.z) + v.y * v.y;
    if squared == 0.0 {
        return Vec3::ZERO;
    }
    v * squared.sqrt().recip()
}

/// The light's view matrix, column-major as the original stores it: the
/// rows are `right = dir × up_hint` and `up = right × dir` (each normalised
/// by the engine's normalise) and `dir`, and the translation is
/// `(−right·origin, −up·origin, −dir·origin)`, so a point maps to its three
/// dot products. The caller passes the light's travel direction and an up
/// hint; `origin` is the point the map centres on.
pub fn light_view_matrix(dir: Vec3, up_hint: Vec3, origin: Vec3) -> Mat4 {
    let right = normalised(dir.cross(up_hint));
    let up = normalised(right.cross(dir));
    Mat4::from_cols(
        Vec4::new(right.x, up.x, dir.x, 0.0),
        Vec4::new(right.y, up.y, dir.y, 0.0),
        Vec4::new(right.z, up.z, dir.z, 0.0),
        Vec4::new(-right.dot(origin), -up.dot(origin), -dir.dot(origin), 1.0),
    )
}

/// The light's orthographic projection for a box spanning
/// `min`..`max`: `2/(max − min)` on the three axes and the translation
/// `−(max + min)/(max − min)`, the depth left in the engine's [−1, 1]
/// convention, column-major.
pub fn orthographic_matrix(min: Vec3, max: Vec3) -> Mat4 {
    let span = max - min;
    let mut m = Mat4::IDENTITY.to_cols_array();
    m[0] = 2.0 / span.x;
    m[5] = 2.0 / span.y;
    m[10] = 2.0 / span.z;
    m[12] = -(max.x + min.x) / span.x;
    m[13] = -(max.y + min.y) / span.y;
    m[14] = -((max.z + min.z) / span.z);
    Mat4::from_cols_array(&m)
}

/// The camera's view volume from its eight frustum corners, cm, in the order
/// Bevy's projection gives them (near bottom right, top right, top left,
/// bottom left, then the far ones): the near and far quads and the four
/// sides, each face with its outward plane.
pub fn frustum_volume(corners: [Vec3; 8]) -> ConvexVolume {
    let [n0, n1, n2, n3, f0, f1, f2, f3] = corners;
    let inside = corners.iter().copied().sum::<Vec3>() / 8.0;
    let faces = [
        [n0, n1, n2, n3],
        [f0, f1, f2, f3],
        [n0, n1, f1, f0],
        [n1, n2, f2, f1],
        [n2, n3, f3, f2],
        [n3, n0, f0, f3],
    ];
    let planes = faces
        .into_iter()
        .map(|face| {
            let mut normal = (face[1] - face[0])
                .cross(face[3] - face[0])
                .normalize_or_zero();
            if normal.dot(inside - face[0]) > 0.0 {
                normal = -normal;
            }
            Plane {
                normal,
                offset: -normal.dot(face[0]),
                vertices: face.to_vec(),
            }
        })
        .collect();
    ConvexVolume { planes }
}

/// The volume's corners in the original's order: each face's vertices, in
/// face order, keeping only those at least [`CORNER_MERGE2`] away (squared,
/// summed `(x² + y²) + z²`) from every corner already kept.
pub fn volume_corners(volume: &ConvexVolume) -> Vec<Vec3> {
    let mut corners: Vec<Vec3> = Vec::new();
    for face in &volume.planes {
        for &p in &face.vertices {
            let duplicate = corners
                .iter()
                .any(|&q| (p - q).length_squared() < CORNER_MERGE2);
            if !duplicate {
                corners.push(p);
            }
        }
    }
    corners
}

/// The fitted region's state across frames: the half extent (what the
/// hysteresis compares against), the snapped centre in the light's frame, the
/// depth the box reaches either way along the light and the softness the
/// update writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunFit {
    /// Half the fitted box's width, cm (the original's quantised extent).
    pub half_extent_cm: f32,
    /// The box's centre in the light's frame, cm.
    pub centre_cm: Vec3,
    /// The softness the update writes, after its 0.15 floor.
    pub softness: f32,
    /// How far the box reaches towards the sun from its centre, cm. The
    /// fitted mode's ±8000 cm; the whole-scene mode's half extent.
    pub near_cm: f32,
    /// How far it reaches away from the sun, cm: ±8000 cm fitted, the
    /// scene's radius whole-scene.
    pub far_cm: f32,
    /// The `c'` the caster bias's depth nudge uses: the light's own softness
    /// source, or the floor's overshoot when the computed softness is below
    /// 0.15.
    pub caster_softness: f32,
}

impl Default for SunFit {
    fn default() -> Self {
        Self {
            half_extent_cm: INITIAL_HALF_EXTENT_CM,
            centre_cm: Vec3::ZERO,
            softness: 0.0,
            near_cm: DEPTH_CM,
            far_cm: DEPTH_CM,
            caster_softness: 0.0,
        }
    }
}

impl SunFit {
    /// Fits the clipped view volume: its corners (kept at least
    /// [`CORNER_MERGE2`] apart) are scaled to metres, the smallest sphere is
    /// fitted in double precision, and the result returns in centimetres,
    /// quantised and snapped. Fewer than two corners change nothing and
    /// return `false`. There is no minimum extent in this mode.
    pub fn fit(&mut self, volume: &ConvexVolume, basis: &LightBasis, map_size: UVec2) -> bool {
        let Some(sphere) = fitted_sphere(volume) else {
            return false;
        };
        self.half_extent_cm = next_half_extent(self.half_extent_cm, sphere.radius);
        self.near_cm = DEPTH_CM;
        self.far_cm = DEPTH_CM;
        let centre_cm = basis.to_light(sphere.centre);
        self.centre_cm = snapped_centre(centre_cm, self.half_extent_cm, map_size);
        true
    }

    /// Covers the whole scene, the update's mode for a level without a
    /// terrain: the scene box's centre rotated into the light's frame, the
    /// box's radius (the length of its half-extent vector) raised to
    /// `min_half_extent_cm` as half extent, the same for the up-sun reach,
    /// `far_cm` raised to the minimum for the down-sun one, and no texel
    /// snap. The softness is `(c + 1)·(200 / half extent)`, with the fitted
    /// mode's 0.15 floor *not* applied.
    pub fn whole_scene(
        &mut self,
        scene: &SceneBox,
        far_cm: f32,
        basis: &LightBasis,
        min_half_extent_cm: f32,
        c: f32,
    ) {
        let radius = scene.extent.length();
        let mut half_extent = radius;
        if min_half_extent_cm > half_extent {
            half_extent = min_half_extent_cm;
        }
        self.half_extent_cm = half_extent;
        self.centre_cm = basis.to_light(scene.centre);
        self.near_cm = half_extent;
        // The engine's max helper is a single `maxss`: the first operand when
        // it is greater, else the second, so a NaN far field gives the
        // minimum (the pseudo-code's `a <= b ? b : a` would keep the NaN).
        self.far_cm = if far_cm > min_half_extent_cm {
            far_cm
        } else {
            min_half_extent_cm
        };
        self.softness = whole_scene_softness(c, half_extent);
        self.caster_softness = c;
    }

    /// The shadow map's matrices and texel size for this fit, in Bevy's
    /// conventions (right-handed view space looking down −Z, reverse-Z
    /// depth): the cascade's world transform, its orthographic projection and
    /// their product. The box spans the centre ± the half extent across the
    /// light and `near_cm` up-sun to `far_cm` down-sun from it.
    pub fn matrices(&self, basis: &LightBasis, map_size: UVec2) -> SunMatrices {
        let centre = basis.from_light(self.centre_cm) / CM_PER_METRE;
        let near = self.near_cm / CM_PER_METRE;
        let far = self.far_cm / CM_PER_METRE;
        let half = self.half_extent_cm / CM_PER_METRE;
        let back = -basis.dir;
        // The view origin sits `near` up-sun of the centre, so the box fills
        // the whole depth range in front of it.
        let eye = centre - near * basis.dir;
        let world_from_cascade = Mat4::from_cols(
            basis.right.extend(0.0),
            basis.up.extend(0.0),
            back.extend(0.0),
            eye.extend(1.0),
        );
        let cascade_from_world = Mat4::from_cols(
            Vec4::new(basis.right.x, basis.up.x, back.x, 0.0),
            Vec4::new(basis.right.y, basis.up.y, back.y, 0.0),
            Vec4::new(basis.right.z, basis.up.z, back.z, 0.0),
            Vec4::new(
                -basis.right.dot(eye),
                -basis.up.dot(eye),
                -back.dot(eye),
                1.0,
            ),
        );
        // Bevy's cascade projection: 2 / width across, and 1 / depth with a
        // translation of 1 for reverse-Z (the near plane at the view origin).
        let half_inv = 1.0 / half;
        let depth_inv = 1.0 / (near + far);
        let clip_from_cascade = Mat4::from_cols(
            Vec4::new(half_inv, 0.0, 0.0, 0.0),
            Vec4::new(0.0, half_inv, 0.0, 0.0),
            Vec4::new(0.0, 0.0, depth_inv, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
        );
        SunMatrices {
            world_from_cascade,
            clip_from_cascade,
            clip_from_world: clip_from_cascade * cascade_from_world,
            texel_size: 2.0 * half / map_size.x.max(1) as f32,
        }
    }

    /// The update's softness step: the sun light's value `c` gives the
    /// filter softness written into the light and the `c'` the caster bias
    /// reads. The fitted mode only; the whole-scene mode sets both in
    /// [`SunFit::whole_scene`].
    pub fn apply_softness(&mut self, c: f32) {
        let softness = sun_softness(c, self.half_extent_cm);
        self.softness = softness.value;
        self.caster_softness = softness.caster;
    }

    /// The render routine's caster bias: the polygon offset it feeds
    /// `glPolygonOffset` and the NDC depth shift it adds to the casters'
    /// projection while drawing the map (see [`CasterBias`]).
    pub fn caster_bias(&self) -> CasterBias {
        caster_bias(
            self.half_extent_cm,
            self.caster_softness,
            self.near_cm + self.far_cm,
        )
    }
}

/// The shadow map's matrices for a fitted region, in Bevy's conventions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunMatrices {
    pub world_from_cascade: Mat4,
    pub clip_from_cascade: Mat4,
    pub clip_from_world: Mat4,
    /// The world size of one shadow map texel, metres.
    pub texel_size: f32,
}

/// The processor's float-to-integer conversion in its default mode: nearest,
/// ties to even. A NaN or a value outside the 64-bit range gives the
/// "integer indefinite", the lowest value.
pub fn convert_nearest(x: f64) -> i64 {
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    let rounded = x.round_ties_even();
    if rounded.is_nan() || rounded >= LIMIT || rounded < -LIMIT {
        return i64::MIN;
    }
    rounded as i64
}

/// The radius quantised to [`STEP_CM`] steps: `200·round(r/200 + 0.4)`.
/// The original divides in single precision, then rounds the widened
/// quotient in double precision before narrowing the step count again.
pub fn quantised_half_extent(radius_cm: f32) -> f32 {
    let steps = f64::from(radius_cm / STEP_CM) + QUANTISE_BIAS;
    (convert_nearest(steps) as f32) * STEP_CM
}

/// The half extent after `radius_cm`: the quantised value when it differs
/// from `current_cm` and the radius is far enough past it (more than 0.3
/// steps above for a grow, more than 0.1 steps below for a shrink), otherwise
/// `current_cm`. The comparisons widen to double precision, as the original's
/// do.
pub fn next_half_extent(current_cm: f32, radius_cm: f32) -> f32 {
    let quantised = quantised_half_extent(radius_cm);
    let radius = f64::from(radius_cm);
    let current = f64::from(current_cm);
    if quantised > current_cm {
        if f64::from(STEP_CM) * GROW_STEPS + current < radius {
            return quantised;
        }
    } else if quantised < current_cm && current - f64::from(STEP_CM) * SHRINK_STEPS > radius {
        return quantised;
    }
    current_cm
}

/// The centre snapped to the texel grid of a `map_size` shadow map: x and y
/// each become `round(c / texel) · texel` with `texel = 2·half_extent / size`,
/// in single precision with the processor's conversion (a zero texel gives
/// −0, as the original's indefinite integer times zero does).
pub fn snapped_centre(centre: Vec3, half_extent_cm: f32, map_size: UVec2) -> Vec3 {
    let mut out = centre;
    for (i, size) in [map_size.x, map_size.y].into_iter().enumerate() {
        let texel = (half_extent_cm * 2.0) / size as f32;
        let steps = convert_nearest(f64::from(out[i] / texel));
        out[i] = (steps as f32) * texel;
    }
    out
}

/// The softness the update writes, and the `c'` the render routine's depth
/// offset then uses. `c` is the sun light's own value (0 in the original's
/// default): `(c + 1)·(400 / half extent)`, floored at [`SOFTNESS_MIN`]
/// (compared in double precision), in which case `c' = 0.15 / (the
/// unfloored softness)` in double precision.
pub fn sun_softness(c: f32, half_extent_cm: f32) -> SunSoftness {
    let raw = (c + 1.0) * (SOFTNESS_CM / half_extent_cm);
    if f64::from(raw) < SOFTNESS_FLOOR {
        SunSoftness {
            caster: (SOFTNESS_FLOOR / f64::from(raw)) as f32,
            value: SOFTNESS_MIN,
        }
    } else {
        SunSoftness {
            caster: c,
            value: raw,
        }
    }
}

/// The softness the whole-scene mode writes: `(c + 1)·(200 / half extent)`,
/// with the fitted mode's 0.15 floor *not* applied and no `c'` derived (the
/// light's own `c` feeds the caster bias as it stands).
pub fn whole_scene_softness(c: f32, half_extent_cm: f32) -> f32 {
    (c + 1.0) * (SOFTNESS_CM / 2.0 / half_extent_cm)
}

/// The sun light's filter softness and the offset scale derived from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunSoftness {
    /// The value the depth offset uses (`c'`).
    pub caster: f32,
    /// The filter softness written into the light (`s`).
    pub value: f32,
}

/// The caster pass's bias, as the render routine computes it before it draws
/// the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CasterBias {
    /// The first argument of the polygon offset: `1 + 1000/h + 2000c'/h`,
    /// each quotient in double precision and the sum stored single, as the
    /// original stores it back before adding the second term.
    pub polygon_factor: f32,
    /// The second argument: 1.0, the smallest offset unit.
    pub polygon_units: f32,
    /// The constant added to the drawn casters' NDC depth while the map is
    /// drawn: `0.01·w`, `w = ½·min((c' + 1)·500 / D, 0.3)` with `D` the
    /// box's near + far reach and the 0.3 compared in double precision. The
    /// original adds it to the projection's z translation and multiplies the
    /// view in, so it shifts the casters (not the receivers) by that much
    /// of the depth range; in a [0, 1] depth range that is half as much
    /// again as in the engine's [−1, 1] one.
    pub depth_nudge: f32,
}

/// The caster pass's polygon offset and depth shift for half extent `h` (cm),
/// the light's softness scale `c'` and the box's near + far reach `depth_cm`.
pub fn caster_bias(half_extent_cm: f32, caster_softness: f32, depth_cm: f32) -> CasterBias {
    // `1/(h·0.001) + 1`, in double precision, stored single; the second term
    // then reads that stored value back widened.
    let first = (1.0 / (f64::from(half_extent_cm) * 0.001) + 1.0) as f32;
    // `(c'·20) / (h·0.01)`: the numerator is a single-precision product, the
    // division in double precision.
    let second = f64::from(caster_softness * 20.0) / (f64::from(half_extent_cm) * 0.01);
    let factor = (f64::from(first) + second) as f32;
    // `w = ((c' + 1)·500) / D`, single precision; above 0.3 (double compare)
    // it is 0.3, then halved.
    let mut w = (1.0 + caster_softness) * 500.0 / depth_cm;
    if f64::from(w) > 0.3 {
        w = 0.3;
    }
    w *= 0.5;
    CasterBias {
        polygon_factor: factor,
        polygon_units: 1.0,
        // `w·0.01` in double precision, as the routine adds it to the
        // projection's z translation (here kept apart and stored single).
        depth_nudge: (f64::from(w) * 0.01) as f32,
    }
}

/// The smallest sphere around a clipped view volume's corners, in
/// centimetres; `None` below two corners.
pub fn fitted_sphere(volume: &ConvexVolume) -> Option<Sphere> {
    let corners = volume_corners(volume);
    if corners.len() < 2 {
        return None;
    }
    let points: Vec<Vec3> = corners.iter().map(|p| *p * POINT_SCALE).collect();
    bounding_sphere(&points).map(|s| Sphere {
        centre: s.centre * CM_PER_METRE,
        radius: s.radius * CM_PER_METRE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    /// An axis-aligned box of half side `h` around `c`, as a frustum.
    fn cube(c: Vec3, h: f32) -> ConvexVolume {
        frustum_volume([
            c + v(-h, -h, -h),
            c + v(h, -h, -h),
            c + v(h, h, -h),
            c + v(-h, h, -h),
            c + v(-h, -h, h),
            c + v(h, -h, h),
            c + v(h, h, h),
            c + v(-h, h, h),
        ])
    }

    #[test]
    fn the_sun_map_is_the_top_detail_size() {
        assert_eq!(crate::SUN_SHADOW_SIZE, 4096);
        assert_eq!(crate::POINT_SHADOW_SIZE, 512);
    }

    #[test]
    fn constants_are_the_originals() {
        assert_eq!(STEP_CM.to_bits(), 0x4348_0000);
        assert_eq!(INITIAL_HALF_EXTENT_CM.to_bits(), 0x453b_8000);
        assert_eq!(DEPTH_CM.to_bits(), 0x45fa_0000);
        assert_eq!(POINT_SCALE.to_bits(), 0x3c23_d70a);
        assert_eq!(CORNER_MERGE2.to_bits(), 0x3dcc_cccd);
        assert_eq!(SOFTNESS_MIN.to_bits(), 0x3e19_999a);
        assert_eq!(SOFTNESS_FLOOR.to_bits(), 0x3fc3_3333_3333_3333);
        assert_eq!(SOFTNESS_CM.to_bits(), 0x43c8_0000);
        assert_eq!(GROW_STEPS.to_bits(), 0x3fd3_3333_3333_3333);
        assert_eq!(SHRINK_STEPS.to_bits(), 0x3fb9_9999_9999_999a);
        assert_eq!(QUANTISE_BIAS.to_bits(), 0x3fd9_9999_9999_999a);
        assert_eq!(FIXED_FRAME_Y.to_bits(), 0x3fef_ff2e_48e8_a71e);
        // 25 000 × 0.5.
        assert_eq!(CAMERA_FAR_CM, 25_000.0 * 0.5);
    }

    #[test]
    fn the_conversion_is_the_processors() {
        assert_eq!(convert_nearest(2.5), 2);
        assert_eq!(convert_nearest(3.5), 4);
        assert_eq!(convert_nearest(-2.5), -2);
        assert_eq!(convert_nearest(f64::NAN), i64::MIN);
        assert_eq!(convert_nearest(f64::INFINITY), i64::MIN);
        assert_eq!(convert_nearest(f64::NEG_INFINITY), i64::MIN);
        assert_eq!(convert_nearest(1e19), i64::MIN);
    }

    #[test]
    fn the_radius_quantises_to_the_nearest_step_plus_0_4() {
        // 200·round(r/200 + 0.4), ties to even. The f32 quotient of 2820/200
        // is 14.100000381…, so 14.5000004 rounds up; 3020/200 likewise.
        assert_eq!(quantised_half_extent(0.0), 0.0); // 0.4 → 0
        assert_eq!(quantised_half_extent(3000.0), 3000.0); // 15.4 → 15
        assert_eq!(quantised_half_extent(3019.0), 3000.0); // 15.495 → 15
        assert_eq!(quantised_half_extent(3020.0), 3200.0); // 15.5000004 → 16
        assert_eq!(quantised_half_extent(2819.0), 2800.0); // 14.495 → 14
        assert_eq!(quantised_half_extent(2820.0), 3000.0); // 14.5000004 → 15
        assert_eq!(quantised_half_extent(2800.0), 2800.0); // 14.4 → 14
        assert_eq!(quantised_half_extent(866.0), 1000.0); // 4.73 → 5
        // 20 cm is 0.1 steps; + 0.4 = 0.5000000015 (the f32 0.1 is a little
        // over), which rounds to 1. 19 cm gives 0.495 → 0.
        assert_eq!(quantised_half_extent(20.0), 200.0);
        assert_eq!(quantised_half_extent(19.0), 0.0);
        // A NaN radius converts to the lowest integer.
        assert_eq!(quantised_half_extent(f32::NAN), (i64::MIN as f32) * STEP_CM);
    }

    #[test]
    fn the_hysteresis_delays_a_grow() {
        // The quantiser asks for 3200 from 3020 on, but the radius must also
        // pass h + 60 = 3060.
        assert_eq!(next_half_extent(3000.0, 3050.0), 3000.0);
        assert_eq!(next_half_extent(3000.0, 3060.0), 3000.0);
        assert_eq!(next_half_extent(3000.0, 3061.0), 3200.0);
        // A shrink follows the quantiser: below 2820, and h − 20 = 2980
        // never holds it back.
        assert_eq!(next_half_extent(3000.0, 2820.0), 3000.0);
        assert_eq!(next_half_extent(3000.0, 2819.0), 2800.0);
        assert_eq!(next_half_extent(3000.0, 1000.0), 1000.0);
        // A NaN radius keeps the half extent (both comparisons fail).
        assert_eq!(next_half_extent(3000.0, f32::NAN), 3000.0);
    }

    #[test]
    fn the_centre_snaps_to_the_texel_grid() {
        // half 1000 cm over 4096 texels: 0.48828125 cm.
        let map = UVec2::new(4096, 4096);
        let snapped = snapped_centre(v(10.0, -10.0, 500.0), 1000.0, map);
        let texel = 2000.0 / 4096.0;
        // 10 / 0.48828125 = 20.48 → 20.
        assert_eq!(snapped.x, 20.0 * texel);
        assert_eq!(snapped.y, -20.0 * texel);
        assert_eq!(snapped.z, 500.0);
        // A tie goes to even: 2.5 texels → 2, 3.5 → 4.
        let snapped = snapped_centre(v(2.5 * texel, 3.5 * texel, 0.0), 1000.0, map);
        assert_eq!(snapped.x, 2.0 * texel);
        assert_eq!(snapped.y, 4.0 * texel);
        // A half extent of zero divides by zero: the indefinite integer
        // times a zero texel is −0.
        let snapped = snapped_centre(v(1.0, -2.0, 3.0), 0.0, map);
        assert_eq!(snapped.x.to_bits(), (-0.0f32).to_bits());
        assert_eq!(snapped.y.to_bits(), (-0.0f32).to_bits());
        assert_eq!(snapped.z, 3.0);
    }

    #[test]
    fn the_light_basis_is_the_originals() {
        // The sun straight overhead: the fixed frame.
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        assert_eq!(basis.dir, Vec3::NEG_Y);
        assert_eq!(basis.right, Vec3::X);
        assert_eq!(basis.up, Vec3::NEG_Z);
        // Straight below gives the same fixed frame.
        assert_eq!(LightBasis::from_towards_sun(Vec3::NEG_Y), basis);
        // Towards (1, 1, 0): dir = (−1/√2, −1/√2, 0); right = dir × Y = −Z;
        // up = right × dir.
        let basis = LightBasis::from_towards_sun(v(1.0, 1.0, 0.0));
        let a = std::f32::consts::FRAC_1_SQRT_2;
        let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-6;
        assert!(close(basis.dir, v(-a, -a, 0.0)));
        assert!(close(basis.right, v(0.0, 0.0, -1.0)));
        assert!(close(basis.up, v(-a, a, 0.0)));
        // Scaling the input does not move the frame.
        let scaled = LightBasis::from_towards_sun(v(7.0, 7.0, 0.0));
        assert!(close(scaled.dir, basis.dir));
        assert!(close(scaled.right, basis.right));
        assert!(close(scaled.up, basis.up));
    }

    #[test]
    fn the_light_frame_round_trips() {
        let basis = LightBasis::from_towards_sun(v(0.3, 1.0, 0.2));
        let p = v(120.0, -45.0, 33.0);
        let there = basis.to_light(p);
        let back = basis.from_light(there);
        assert!((back - p).length() < 1e-4);
    }

    #[test]
    fn the_planes_follow_the_cameras_view() {
        let scene = SceneBox::from_bounds(v(-1000.0, -200.0, -500.0), v(3000.0, 400.0, 1500.0));
        let vertical = VerticalRange {
            high: 340.0,
            low: 0.0,
        };
        // A camera looking towards −X, down and towards +Z: its backward axis
        // is (+, +, −).
        let planes = view_clip_planes(&scene, vertical, v(0.5, 0.7, -0.5));
        assert_eq!(
            planes,
            vec![
                // Looking towards −X: keep x ≥ the box's low edge, −1000.
                ClipPlane::new(Vec3::NEG_X, 1000.0),
                // Looking down: keep y ≥ the floor.
                ClipPlane::new(Vec3::NEG_Y, -0.0),
                // Looking towards +Z: keep z ≤ the box's high edge, 1500.
                ClipPlane::new(Vec3::Z, 1500.0),
            ]
        );
        // A zero (or NaN) component adds no plane.
        let planes = view_clip_planes(&scene, vertical, v(0.0, -1.0, f32::NAN));
        assert_eq!(planes, vec![ClipPlane::new(Vec3::Y, 340.0)]);
    }

    #[test]
    fn without_a_record_the_vertical_planes_are_the_boxs_x_edges() {
        let scene = SceneBox::from_bounds(v(-1000.0, -200.0, -500.0), v(3000.0, 400.0, 1500.0));
        assert_eq!(
            VerticalRange::of_level(&scene, None),
            VerticalRange {
                high: 3000.0,
                low: -1000.0
            }
        );
        assert_eq!(
            VerticalRange::of_level(&scene, Some((-500.0, 800.0))),
            VerticalRange {
                high: 800.0,
                low: -500.0
            }
        );
        assert_eq!(
            slab_clip_planes(VerticalRange {
                high: 340.0,
                low: 0.0
            }),
            [
                ClipPlane::new(Vec3::Y, 340.0),
                ClipPlane::new(Vec3::NEG_Y, -0.0),
            ]
        );
    }

    #[test]
    fn a_box_clipped_by_a_middle_plane_keeps_its_corners() {
        // A 1000 cm cube around the origin cut by x ≥ −500 (its own face):
        // the volume is unchanged and still has eight corners.
        let mut volume = cube(Vec3::ZERO, 500.0);
        assert_eq!(volume_corners(&volume).len(), 8);
        ClipPlane::new(Vec3::NEG_X, 500.0).clip(&mut volume);
        assert_eq!(volume_corners(&volume).len(), 8);
        // Cutting through the middle leaves the eight corners of the half.
        ClipPlane::new(Vec3::X, 0.0).clip(&mut volume);
        let corners = volume_corners(&volume);
        assert_eq!(corners.len(), 8);
        assert!(corners.iter().all(|p| p.x <= 0.0));
        // Four corners keep x = −500 and four sit on the cut at x = 0.
        assert_eq!(corners.iter().filter(|p| p.x == -500.0).count(), 4);
        assert_eq!(corners.iter().filter(|p| p.x == 0.0).count(), 4);
        for &p in &corners {
            assert_eq!(p.y.abs(), 500.0);
            assert_eq!(p.z.abs(), 500.0);
        }
    }

    #[test]
    fn the_frustum_planes_face_outward() {
        let volume = cube(v(10.0, 20.0, 30.0), 500.0);
        for plane in &volume.planes {
            assert!(plane.distance(v(10.0, 20.0, 30.0)) < 0.0);
            for &p in &plane.vertices {
                assert!(plane.distance(p).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn a_fit_of_a_cube_is_its_sphere() {
        // A 1000 cm cube's corners lie on a sphere of radius √3·500.
        let sphere = fitted_sphere(&cube(Vec3::ZERO, 500.0)).unwrap();
        assert!(sphere.centre.length() < 1e-4);
        let radius = 3f32.sqrt() * 500.0;
        assert!((sphere.radius - radius).abs() < 1e-3);
        // The steps: 866.025/200 + 0.4 = 4.73 → 5 steps = 1000 cm.
        assert_eq!(quantised_half_extent(sphere.radius), 1000.0);
    }

    #[test]
    fn fewer_than_two_corners_fit_nothing() {
        let empty = ConvexVolume::default();
        let mut fit = SunFit::default();
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        assert!(!fit.fit(&empty, &basis, UVec2::splat(4096)));
        assert_eq!(fit, SunFit::default());
        // A volume whose corners all merge into one point.
        let point = ConvexVolume {
            planes: vec![Plane {
                normal: Vec3::Y,
                offset: 0.0,
                vertices: vec![v(10.0, 10.0, 10.0); 4],
            }],
        };
        assert!(!fit.fit(&point, &basis, UVec2::splat(4096)));
        assert_eq!(fit, SunFit::default());
    }

    #[test]
    fn a_view_clipped_by_the_scene_fits_its_sphere() {
        // A 2000 cm cube at the origin seen by a camera looking straight
        // down (backward axis +Y), in a level whose floor is at 100 cm: the
        // plane y ≥ 100 keeps four corners at y = 1000 and four cut in at
        // y = 100.
        let mut volume = cube(Vec3::ZERO, 1000.0);
        let scene = SceneBox {
            centre: Vec3::ZERO,
            extent: Vec3::splat(100_000.0),
        };
        let vertical = VerticalRange::of_level(&scene, Some((100.0, 340.0)));
        for plane in view_clip_planes(&scene, vertical, Vec3::Y) {
            plane.clip(&mut volume);
        }
        let sphere = fitted_sphere(&volume).unwrap();
        // Centre (0, 550, 0) cm; r² = 1000² + 450² + 1000².
        assert!((sphere.centre - v(0.0, 550.0, 0.0)).length() < 1e-2);
        let radius = (2.0f32 * 1000.0 * 1000.0 + 450.0 * 450.0).sqrt();
        assert!((sphere.radius - radius).abs() < 1e-2);
        // In the light's frame (sun overhead: right +X, up −Z, dir −Y) the
        // centre is (0, 0, −550); 1484.1/200 + 0.4 = 7.82 → 8 steps, and
        // from 3000 the shrink is taken (1484.1 < 2980).
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let mut fit = SunFit::default();
        assert!(fit.fit(&volume, &basis, UVec2::splat(4096)));
        assert_eq!(fit.half_extent_cm, 1600.0);
        // x and y snap to the 0.78125 cm grid; the depth is not snapped.
        assert_eq!(fit.centre_cm.x, 0.0);
        assert_eq!(fit.centre_cm.y.abs(), 0.0);
        assert!((fit.centre_cm.z + 550.0).abs() < 1e-2);
    }

    #[test]
    fn a_volume_wholly_outside_a_plane_fits_nothing() {
        let mut volume = cube(v(0.0, 1200.0, 0.0), 300.0);
        ClipPlane::new(Vec3::Y, 800.0).clip(&mut volume);
        assert!(volume.planes.is_empty());
        assert!(volume_corners(&volume).is_empty());
        let mut fit = SunFit::default();
        assert!(!fit.fit(
            &volume,
            &LightBasis::from_towards_sun(Vec3::Y),
            UVec2::splat(4096)
        ));
        assert_eq!(fit.half_extent_cm, INITIAL_HALF_EXTENT_CM);
    }

    #[test]
    fn the_fit_quantises_and_snaps() {
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let mut fit = SunFit::default();
        assert!(fit.fit(&cube(Vec3::ZERO, 500.0), &basis, UVec2::splat(4096)));
        // (866.025)/200 + 0.4 = 4.73 → 5 steps.
        assert_eq!(fit.half_extent_cm, 1000.0);
        // The centre is the origin: already on the grid.
        assert_eq!(fit.centre_cm.abs(), Vec3::ZERO);
    }

    #[test]
    fn the_matrices_cover_the_fitted_box() {
        let fit = SunFit {
            half_extent_cm: 1000.0,
            centre_cm: Vec3::ZERO,
            ..Default::default()
        };
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let matrices = fit.matrices(&basis, UVec2::splat(4096));
        let clip = |light: Vec3| {
            let world = basis.from_light(light) / CM_PER_METRE;
            matrices.clip_from_world * world.extend(1.0)
        };
        // The centre lands in the middle of the map, halfway through
        // reverse-Z's depth range.
        let centre = clip(fit.centre_cm);
        assert!(centre.x.abs() < 1e-6 && centre.y.abs() < 1e-6);
        assert!((centre.z - 0.5).abs() < 1e-6);
        // The box's sides land on the map's edges.
        let right = clip(fit.centre_cm + v(1000.0, 0.0, 0.0));
        assert!((right.x - 1.0).abs() < 1e-6);
        let up = clip(fit.centre_cm + v(0.0, 1000.0, 0.0));
        assert!((up.y - 1.0).abs() < 1e-6);
        // Down-sun is the far plane (depth 0), up-sun the near plane (1).
        let far = clip(fit.centre_cm + v(0.0, 0.0, DEPTH_CM));
        assert!(far.z.abs() < 1e-6);
        let near = clip(fit.centre_cm - v(0.0, 0.0, DEPTH_CM));
        assert!((near.z - 1.0).abs() < 1e-6);
        assert!((matrices.texel_size - 2000.0 / 100.0 / 4096.0).abs() < 1e-9);
        // The world transform is the inverse of the view.
        let view = matrices.clip_from_cascade.inverse() * matrices.clip_from_world;
        let identity = view * matrices.world_from_cascade;
        assert!(identity.abs_diff_eq(Mat4::IDENTITY, 1e-5));
    }

    #[test]
    fn the_softness_floors_at_0_15() {
        // 400/h above 0.15 means h below 2666.67 cm: at 1000 cm it is 0.4.
        let soft = sun_softness(0.0, 1000.0);
        assert_eq!(soft.value, 0.4);
        assert_eq!(soft.caster, 0.0);
        // 400/3000 = 0.1333 takes the floor, c' = 0.15/0.1333 = 1.125.
        let soft = sun_softness(0.0, 3000.0);
        assert_eq!(soft.value, 0.15);
        assert!((soft.caster - 1.125).abs() < 1e-6);
        // A huge extent: 400/16000 = 0.025 → 0.15, c' = 6.
        let soft = sun_softness(0.0, 16000.0);
        assert_eq!(soft.value, 0.15);
        assert!((soft.caster - 6.0).abs() < 1e-5);
    }

    #[test]
    fn the_fit_has_no_minimum_extent() {
        // A 200 cm cube: r = 173.2, 173.2/200 + 0.4 = 1.27 → 1 step. From
        // 3000 the shrink is taken (173.2 < 2980): 200 cm, well below the
        // whole-scene mode's 1 000 cm minimum, which this mode never reads.
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let mut fit = SunFit::default();
        assert!(fit.fit(&cube(Vec3::ZERO, 100.0), &basis, UVec2::splat(4096)));
        assert_eq!(fit.half_extent_cm, 200.0);
        assert_eq!((fit.near_cm, fit.far_cm), (DEPTH_CM, DEPTH_CM));
    }

    #[test]
    fn the_whole_scene_mode_uses_the_scene_radius_and_no_snap() {
        // A 4 000 × 1 000 × 3 000 cm box around (100, 200, 300): its radius
        // is √(2000² + 500² + 1500²) = 2554.4 cm.
        let scene = SceneBox {
            centre: v(100.0, 200.0, 300.0),
            extent: v(2000.0, 500.0, 1500.0),
        };
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let mut fit = SunFit::default();
        // The far field is the scene's own radius from the node unit, here
        // modelled as the same half-diagonal.
        fit.whole_scene(
            &scene,
            scene.extent.length(),
            &basis,
            MIN_HALF_EXTENT_CM,
            0.0,
        );
        let radius = (2000f32 * 2000.0 + 500.0 * 500.0 + 1500.0 * 1500.0).sqrt();
        assert_eq!(fit.half_extent_cm, radius);
        assert_eq!(fit.near_cm, radius);
        assert_eq!(fit.far_cm, radius);
        // The centre is rotated into the light's frame: the sun overhead has
        // right +X, up −Z, dir −Y, so (100, 200, 300) → (100, −300, −200).
        assert!((fit.centre_cm - v(100.0, -300.0, -200.0)).length() < 1e-3);
        // No texel snap: an off-grid centre stays.
        assert!((fit.centre_cm.x - 100.0).abs() < 1e-3);
        // Softness: (c + 1)·(200 / h), no 0.15 floor.
        let expected = 200.0 / radius;
        assert!((fit.softness - expected).abs() < 1e-6);
        assert_eq!(fit.caster_softness, 0.0);
        // A tiny scene takes the minimum extent for its half extent, its
        // near reach and its far field.
        let tiny = SceneBox {
            centre: Vec3::ZERO,
            extent: v(100.0, 100.0, 100.0),
        };
        let mut fit = SunFit::default();
        fit.whole_scene(&tiny, 10.0, &basis, MIN_HALF_EXTENT_CM, 0.0);
        assert_eq!(fit.half_extent_cm, 1000.0);
        assert_eq!(fit.near_cm, 1000.0);
        assert_eq!(fit.far_cm, 1000.0);
        assert_eq!(fit.softness, 0.2);
        // A NaN far field gives the minimum (`maxss` returns its second
        // operand on NaN), and a NaN minimum is that second operand too.
        fit.whole_scene(&tiny, f32::NAN, &basis, MIN_HALF_EXTENT_CM, 0.0);
        assert_eq!(fit.far_cm, 1000.0);
        fit.whole_scene(&tiny, 2500.0, &basis, f32::NAN, 0.0);
        assert!(fit.far_cm.is_nan());
        // The half extent's raise is `min > h`: a NaN minimum never raises.
        assert_eq!(fit.half_extent_cm, v(100.0, 100.0, 100.0).length());
    }

    #[test]
    fn the_whole_scene_box_is_the_radius_across_and_near_to_far_along() {
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let scene = SceneBox {
            centre: v(0.0, 100.0, 0.0),
            extent: v(2000.0, 500.0, 2000.0),
        };
        let mut fit = SunFit::default();
        fit.whole_scene(&scene, 2000.0, &basis, MIN_HALF_EXTENT_CM, 0.0);
        let matrices = fit.matrices(&basis, UVec2::splat(4096));
        let clip = |light: Vec3| {
            let world = basis.from_light(light) / CM_PER_METRE;
            matrices.clip_from_world * world.extend(1.0)
        };
        // The depth range is near + far = the radius + the far field.
        let h = fit.half_extent_cm;
        let centre = clip(fit.centre_cm);
        assert!((centre.z - fit.far_cm / (fit.near_cm + fit.far_cm)).abs() < 1e-5);
        // Up-sun by the half extent is the near plane, down-sun by the far
        // field the far plane.
        let near = clip(fit.centre_cm - v(0.0, 0.0, h));
        assert!((near.z - 1.0).abs() < 1e-5);
        let far = clip(fit.centre_cm + v(0.0, 0.0, fit.far_cm));
        assert!(far.z.abs() < 1e-5);
        // Across the light the box is the radius each way.
        let right = clip(fit.centre_cm + v(h, 0.0, 0.0));
        assert!((right.x - 1.0).abs() < 1e-5);
        assert!((matrices.texel_size - 2.0 * h / 100.0 / 4096.0).abs() < 1e-9);
    }

    #[test]
    fn the_caster_bias_is_the_originals() {
        // h = 1000, c' = 0: factor = 1 + 1 = 2; w = 500/16000 = 0.03125,
        // halved to 0.015625, nudged by 0.01 → 1.5625e-4.
        let bias = caster_bias(1000.0, 0.0, 16000.0);
        assert_eq!(bias.polygon_factor, 2.0);
        assert_eq!(bias.polygon_units, 1.0);
        assert!((bias.depth_nudge - 1.5625e-4).abs() < 1e-9);
        // A floored softness c' = 1.125 at h = 3000: factor = 1 + 1/3 +
        // 2000·1.125/3000 = 1.3333 + 0.75 = 2.0833; w = 2.125·500/16000 =
        // 0.06640625, halved → 0.033203125 → 3.3203e-4.
        let bias = caster_bias(3000.0, 1.125, 16000.0);
        assert!((bias.polygon_factor - (1.0 + 1.0 / 3.0 + 0.75)).abs() < 1e-6);
        assert!((bias.depth_nudge - 3.3203125e-4).abs() < 1e-9);
        // w above 0.3 is clamped (a 200 cm box: 500/1600 = 0.3125 → 0.3),
        // so the nudge is 0.0015.
        let bias = caster_bias(200.0, 0.0, 400.0);
        assert!((bias.depth_nudge - 0.0015).abs() < 1e-9);
        assert_eq!(bias.polygon_factor, 1.0 + 1000.0 / 200.0);
        // The fit carries the values: at h = 1000 cm (c' = 0, no floor) the
        // bias is the 2.0 above; at 3000 cm the softness floors and gives
        // c' = 1.125.
        let mut fit = SunFit {
            half_extent_cm: 1000.0,
            ..Default::default()
        };
        fit.apply_softness(0.0);
        assert_eq!(fit.caster_softness, 0.0);
        assert_eq!(fit.caster_bias().polygon_factor, 2.0);
        assert!((fit.caster_bias().depth_nudge - 1.5625e-4).abs() < 1e-9);
        fit.half_extent_cm = 3000.0;
        fit.apply_softness(0.0);
        assert_eq!(fit.caster_softness, 1.125);
        assert!((fit.caster_bias().polygon_factor - (1.0 + 1.0 / 3.0 + 0.75)).abs() < 1e-5);
    }

    #[test]
    fn the_light_view_matrix_holds_the_basis_as_rows() {
        // dir -Z with the up hint +Y: right = -Z × Y = +X, up = X × -Z = +Y.
        let origin = Vec3::new(10.0, 20.0, 30.0);
        let m = light_view_matrix(Vec3::NEG_Z, Vec3::Y, origin);
        assert_eq!(m.row(0), Vec4::new(1.0, 0.0, 0.0, -10.0));
        assert_eq!(m.row(1), Vec4::new(0.0, 1.0, 0.0, -20.0));
        assert_eq!(m.row(2), Vec4::new(0.0, 0.0, -1.0, 30.0));
        // A tilted direction: the rows (not the columns) are the basis, so
        // a point maps to its dot products with right, up and dir.
        let dir = Vec3::new(0.0, -0.6, -0.8);
        let m = light_view_matrix(dir, Vec3::Y, Vec3::ZERO);
        let right = normalised(dir.cross(Vec3::Y));
        let up = normalised(right.cross(dir));
        let p = Vec3::new(3.0, 5.0, 7.0);
        let v = m * p.extend(1.0);
        assert_eq!(v.truncate(), Vec3::new(right.dot(p), up.dot(p), dir.dot(p)));
        assert_eq!(m.x_axis, Vec4::new(right.x, up.x, dir.x, 0.0));
    }

    #[test]
    fn the_orthographic_matrix_maps_the_box_to_the_unit_cube() {
        let min = Vec3::new(-100.0, -50.0, -200.0);
        let max = Vec3::new(100.0, 150.0, 200.0);
        let m = orthographic_matrix(min, max);
        assert_eq!(m.x_axis.x, 2.0 / 200.0);
        assert_eq!(m.y_axis.y, 2.0 / 200.0);
        assert_eq!(m.z_axis.z, 2.0 / 400.0);
        assert_eq!(m.w_axis.x, 0.0);
        assert_eq!(m.w_axis.y, -0.5);
        assert_eq!(m.w_axis.z, 0.0);
        let p = m * Vec4::new(100.0, 150.0, 200.0, 1.0);
        assert!((p.x - 1.0).abs() < 1e-6);
        assert!((p.y - 1.0).abs() < 1e-6);
        assert!((p.z - 1.0).abs() < 1e-6);
    }
}
