//! The fitted orthographic region of the original's single sun shadow map.
//!
//! The original keeps one orthographic depth map for the sun, not a cascade
//! chain. Every frame its update copies the active view's convex volume, cuts
//! it by up to three planes on the side the sun comes from, fits the smallest
//! sphere around the surviving corners ([`crate::bound_sphere`]), converts
//! that sphere to centimetres, quantises its radius to 200 cm steps with a
//! hysteresis, snaps the centre to the shadow map's texel grid and gives the
//! box a fixed ±8000 cm depth. The quantised radius carries over to the next
//! frame, which is what the hysteresis compares against.
//!
//! The maths here is the original's, in the original's order:
//!
//! - The corners are merged within 0.1 cm² and scaled by 0.01 (the
//!   centimetre world into the metre units the sphere solver is tuned for),
//!   and the fitted centre and radius are multiplied by 100 again.
//! - The radius becomes `200·round(r/200 + 0.4)`; the conversion is the
//!   processor's `cvtsd2si` (`round_ties_even`, the default rounding mode).
//!   The engine's `floor`/`ceil` helpers are separate routines this path
//!   never calls. A step is taken only when the new value differs *and* the
//!   radius is more than 0.3 steps above (grow) or 0.1 steps below (shrink)
//!   the current half extent.
//! - The centre is rotated into the light's basis and its x and y are
//!   snapped to the texel grid (`2·half_extent / map size`), again rounding
//!   to nearest. The grid is anchored at the light-space origin, as the
//!   original's is.
//! - Fewer than two corners fit nothing: the previous frame's projection
//!   stays, which [`SunFit::fit`] reports.
//!
//! The clipping follows the engine's convex-volume cut: each face keeps the
//! part where `normal · p − w ≤ 0`, the cut segments are chained into a cap
//! polygon from the first segment on (turned round when the first face's last
//! vertex is inside), and faces left with fewer than three points are dropped.
//! `exa_physics::intersect` ports the same routine for shadows and clipping
//! in the simulation; this one keeps only the polygons, which is all the fit
//! reads.
//!
//! Not reproduced here: the softness the update writes into the sun's light
//! (see [`sun_softness`]; Bevy's directional shadows have no such uniform
//! without the PCSS feature) and the caster-side polygon offset and NDC
//! depth nudge of the render routine. Rules: `docs/RENDERING.md` (Sun
//! shadow fit).
//!
//! Evidence: the procedure listings and decompiler dump of the component, the
//! vector helpers they call (distance, length, normalise, matrix row/point
//! transforms, min/max, the float-to-int conversions and the engine's
//! floor/ceil helpers), and the level loader that fills the scene box.

// The original's branches take their else side on an unordered comparison, so
// `!(d <= 0.0)` counts a NaN distance as outside. Kept as written.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Mat4, UVec2, Vec3, Vec4};

use crate::bound_sphere::{Sphere, bounding_sphere};

/// The step the fitted half extent is quantised to, cm (the original's
/// `200.0`).
pub const STEP_CM: f32 = 200.0;
/// The half extent before the first fit, cm: the shadow map's constructor
/// default, and the value the first hysteresis compares against.
pub const INITIAL_HALF_EXTENT_CM: f32 = 3000.0;
/// How far the box reaches on each side of its centre along the light, cm.
pub const DEPTH_CM: f32 = 8000.0;
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
/// Chained cap ends this close (squared, cm², compared in double precision)
/// close the cap.
const CAP_CLOSED2: f64 = 1e-4;
/// Two cut ends this close (squared, cm²) are the same point when a cap is
/// chained.
const CAP_SAME_POINT2: f32 = 5.0;
/// The vertical centre of the fitted mode's scene box, cm: the original sets
/// the scene box's vertical centre to 1.5 m before clipping.
pub const SCENE_MIDDLE_CM: f32 = 150.0;
/// The vertical half extent of the fitted mode's scene box, cm: with
/// [`SCENE_MIDDLE_CM`] the sun's planes keep y inside −500…800 cm.
pub const SCENE_HALF_HEIGHT_CM: f32 = 650.0;
/// The softness the update writes when the computed one is below it (the
/// original's `0.15`).
pub const SOFTNESS_MIN: f32 = 0.15;
/// The softness is `(c + 1) · 400 / half extent` (the original's fixed 400 cm
/// divided by the fitted half extent).
pub const SOFTNESS_CM: f32 = 400.0;
/// Centimetres in a metre.
const CM_PER_METRE: f32 = 100.0;

/// A half-space the visible region is clipped by: a point is kept when
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

    /// The signed distance: positive outside.
    fn distance(&self, p: Vec3) -> f32 {
        // The original multiplies and adds x, y, z in that order, then
        // subtracts w.
        self.normal.x * p.x + self.normal.y * p.y + self.normal.z * p.z - self.w
    }
}

/// The scene's axis-aligned bounds as the fitted mode clips the visible
/// region by them, cm.
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
}

/// The one plane per axis the fitted mode clips by: on an axis the sun
/// travels along negatively the plane at the scene box's high edge is kept
/// (`axis ≤ centre + extent`), along positively the one at its low edge
/// (`axis ≥ centre − extent`), so the region behind the sun is cut away. A
/// zero component adds no plane. The vertical axis uses the original's fixed
/// box ([`SCENE_MIDDLE_CM`], [`SCENE_HALF_HEIGHT_CM`]) in place of the
/// scene's own.
pub fn direction_clip_planes(scene: SceneBox, dir: Vec3) -> Vec<ClipPlane> {
    let centre = Vec3::new(scene.centre.x, SCENE_MIDDLE_CM, scene.centre.z);
    let extent = Vec3::new(scene.extent.x, SCENE_HALF_HEIGHT_CM, scene.extent.z);
    let mut planes = Vec::with_capacity(3);
    for axis in 0..3 {
        if dir[axis] < 0.0 {
            let mut normal = Vec3::ZERO;
            normal[axis] = 1.0;
            planes.push(ClipPlane::new(normal, centre[axis] + extent[axis]));
        } else if dir[axis] > 0.0 {
            let mut normal = Vec3::ZERO;
            normal[axis] = -1.0;
            planes.push(ClipPlane::new(normal, extent[axis] - centre[axis]));
        }
    }
    planes
}

/// The two vertical planes of the original's other mode (a camera mode
/// beyond the first two): `y ≤ high` and `y ≥ low`, cm. The level loader
/// sets `low` and `high` from the room's floor and ceiling (0 and 340 cm in
/// the shipped grid levels).
pub fn slab_clip_planes(low_cm: f32, high_cm: f32) -> [ClipPlane; 2] {
    [
        ClipPlane::new(Vec3::Y, high_cm),
        ClipPlane::new(Vec3::NEG_Y, -low_cm),
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
        // The engine's branch: past 0.9999 (and only then) it takes the fixed
        // frame; a NaN |y| takes the regular path.
        if dir.y.abs() > 0.9999 {
            // The original's fixed frame for |dir.y| > 0.9999 (a sun
            // straight above): right +X, up −Z, forward down.
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

/// A convex region given by the vertex loops of its faces, cm.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Region {
    faces: Vec<Vec<Vec3>>,
}

impl Region {
    /// A region of the given face loops.
    pub fn from_faces(faces: Vec<Vec<Vec3>>) -> Self {
        Self { faces }
    }

    /// The camera's view volume: its near and far quads and four sides, from
    /// the eight corners in the order the projection gives them (near bottom
    /// right, top right, top left, bottom left, then the far ones).
    pub fn frustum(corners: [Vec3; 8]) -> Self {
        let [n0, n1, n2, n3, f0, f1, f2, f3] = corners;
        Self::from_faces(vec![
            vec![n0, n1, n2, n3],
            vec![f0, f1, f2, f3],
            vec![n0, n1, f1, f0],
            vec![n1, n2, f2, f1],
            vec![n2, n3, f3, f2],
            vec![n3, n0, f0, f3],
        ])
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Cuts the region by `plane`, keeping the inner side. Each face keeps
    /// its vertices inside the plane and the crossing points, and is dropped
    /// when fewer than three remain. The cut segments become the cap face,
    /// chained from the first segment on: a cap whose ends meet is closed,
    /// otherwise it is also chained from its start. The cap is appended last.
    pub fn clip(&mut self, plane: ClipPlane) {
        let distance = |p: Vec3| plane.distance(p);
        let mut faces = Vec::with_capacity(self.faces.len() + 1);
        let mut segments: Vec<[Vec3; 2]> = Vec::new();
        let mut turn_first = false;
        for (i, face) in self.faces.iter().enumerate() {
            let cut = cut(face, distance);
            if i == 0 && !face.is_empty() {
                turn_first = cut.last_inside;
            }
            segments.extend(cut.segment);
            if cut.points.len() >= 3 {
                faces.push(cut.points);
            }
        }
        if segments.len() >= 2 {
            let [a, b] = segments[0];
            let mut cap = if turn_first { vec![b, a] } else { vec![a, b] };
            chain(&mut segments, &mut cap);
            let ends = (cap[0] - cap[cap.len() - 1]).length_squared();
            if f64::from(ends) < CAP_CLOSED2 {
                cap.pop();
            } else {
                cap.reverse();
                chain(&mut segments, &mut cap);
                cap.reverse();
            }
            if cap.len() >= 3 {
                faces.push(cap);
            }
        }
        self.faces = faces;
    }

    /// The region's corners in the original's order: each face's vertices,
    /// in face order, keeping only those at least [`CORNER_MERGE2`] away
    /// (squared) from every corner already kept.
    pub fn corners(&self) -> Vec<Vec3> {
        let mut corners: Vec<Vec3> = Vec::new();
        for face in &self.faces {
            for &p in face {
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
}

/// One polygon cut by a plane.
struct Cut {
    /// The part on the inner side, crossing points included.
    points: Vec<Vec3>,
    /// The two crossings: entry then exit when the polygon's last vertex is
    /// outside, exit then entry when it is inside.
    segment: Option<[Vec3; 2]>,
    /// Whether the polygon's last vertex is inside.
    last_inside: bool,
}

/// Cuts a convex polygon by `distance` (inside ≤ 0), walking it from the last
/// vertex's side. A vertex on the plane is kept once and makes its crossing;
/// a NaN distance is outside. An empty polygon gives nothing.
fn cut(polygon: &[Vec3], distance: impl Fn(Vec3) -> f32) -> Cut {
    let Some(&last) = polygon.last() else {
        return Cut {
            points: Vec::new(),
            segment: None,
            last_inside: false,
        };
    };
    let crossing = |a: Vec3, b: Vec3, t: f32| a + (b - a) * t;
    let n = polygon.len();
    let mut points = Vec::with_capacity(n + 1);
    let mut previous = n - 1;
    let mut before = distance(last);
    let last_inside = before <= 0.0;
    let mut first = None;
    let mut second = None;
    if !last_inside {
        // Outside to start: find the entry, keep the inner run, stop at the
        // exit.
        for (k, &vertex) in polygon.iter().enumerate() {
            let d = distance(vertex);
            if d <= 0.0 {
                first = Some(if d < 0.0 {
                    let p = crossing(polygon[previous], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    vertex
                });
                points.push(vertex);
                previous = k;
                before = d;
                break;
            }
            previous = k;
            before = d;
        }
        for (k, &vertex) in polygon.iter().enumerate().skip(previous + 1) {
            let d = distance(vertex);
            if !(d <= 0.0) {
                second = Some(if before < 0.0 {
                    let p = crossing(polygon[previous], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    polygon[previous]
                });
                break;
            }
            points.push(vertex);
            previous = k;
            before = d;
        }
    } else {
        // Inside to start: keep the run up to the exit, skip the outer run,
        // keep the rest from the entry.
        for (k, &vertex) in polygon.iter().enumerate() {
            let d = distance(vertex);
            if !(d <= 0.0) {
                first = Some(if before < 0.0 {
                    let p = crossing(polygon[previous], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    polygon[previous]
                });
                previous = k;
                before = d;
                break;
            }
            points.push(vertex);
            previous = k;
            before = d;
        }
        for (k, &vertex) in polygon.iter().enumerate().skip(previous + 1) {
            let d = distance(vertex);
            if d <= 0.0 {
                second = Some(if d < 0.0 {
                    let p = crossing(polygon[k - 1], vertex, before / (before - d));
                    points.push(p);
                    p
                } else {
                    vertex
                });
                previous = k - 1;
                break;
            }
            before = d;
        }
        points.extend_from_slice(&polygon[previous + 1..]);
    }
    let segment = match (first, second) {
        (Some(a), Some(b)) => Some([a, b]),
        _ => None,
    };
    Cut {
        points,
        segment,
        last_inside,
    }
}

/// Whether two cut ends are the same point for chaining.
fn same_point(a: Vec3, b: Vec3) -> bool {
    (a - b).length_squared() < CAP_SAME_POINT2
}

/// Chains the cut segments into the cap polygon, from the first segment on:
/// each round takes the first remaining segment with an end at the chain's
/// end, appends its other end and moves the last segment into its place.
fn chain(segments: &mut Vec<[Vec3; 2]>, points: &mut Vec<Vec3>) {
    let mut i = 1;
    while i < segments.len() {
        // A chain is seeded with two points and only grows; the guard keeps
        // the helper total all the same.
        let Some(&end) = points.last() else { break };
        let [a, b] = segments[i];
        let next = if same_point(a, end) {
            b
        } else if same_point(b, end) {
            a
        } else {
            i += 1;
            continue;
        };
        points.push(next);
        segments.swap_remove(i);
        i = 1;
    }
}

/// The fitted region's state across frames: the half extent (what the
/// hysteresis compares against), the snapped centre in the light's frame and
/// the softness the update writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunFit {
    /// Half the fitted box's width, cm (the original's quantised extent).
    pub half_extent_cm: f32,
    /// The box's centre in the light's frame, cm.
    pub centre_cm: Vec3,
    /// The softness the update writes, after its 0.15 floor.
    pub softness: f32,
}

impl Default for SunFit {
    fn default() -> Self {
        Self {
            half_extent_cm: INITIAL_HALF_EXTENT_CM,
            centre_cm: Vec3::ZERO,
            softness: 0.0,
        }
    }
}

impl SunFit {
    /// Fits the region: its corners (kept at least [`CORNER_MERGE2`] apart)
    /// are scaled to metres, the smallest sphere is fitted in double
    /// precision, and the result returns in centimetres, quantised and
    /// snapped. Fewer than two corners change nothing and return `false`.
    pub fn fit(&mut self, region: &Region, basis: &LightBasis, map_size: UVec2) -> bool {
        let corners = region.corners();
        if corners.len() < 2 {
            return false;
        }
        let points: Vec<Vec3> = corners.iter().map(|p| *p * POINT_SCALE).collect();
        let Some(sphere) = bounding_sphere(&points) else {
            return false;
        };
        let radius_cm = sphere.radius * CM_PER_METRE;
        self.half_extent_cm = next_half_extent(self.half_extent_cm, radius_cm);
        let centre_cm = basis.to_light(sphere.centre * CM_PER_METRE);
        self.centre_cm = snapped_centre(centre_cm, self.half_extent_cm, map_size);
        true
    }

    /// The shadow map's matrices and texel size for this fit, in Bevy's
    /// conventions (right-handed view space looking down −Z, reverse-Z
    /// depth): the cascade's world transform, its orthographic projection and
    /// their product. The box spans the centre ± the half extent across the
    /// light and ±[`DEPTH_CM`] along it.
    pub fn matrices(&self, basis: &LightBasis, map_size: UVec2) -> SunMatrices {
        let centre = basis.from_light(self.centre_cm) / CM_PER_METRE;
        let depth = DEPTH_CM / CM_PER_METRE;
        let half = self.half_extent_cm / CM_PER_METRE;
        let back = -basis.dir;
        // The view origin sits one depth behind the centre (up-sun), so the
        // whole ±depth box is in front of it.
        let eye = centre - depth * basis.dir;
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
        let depth_inv = 1.0 / (2.0 * depth);
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

/// The radius quantised to [`STEP_CM`] steps: `200·round(r/200 + 0.4)`.
/// The original divides in single precision, then rounds the widened
/// quotient in double precision before narrowing the step count again.
pub fn quantised_half_extent(radius_cm: f32) -> f32 {
    let steps = f64::from(radius_cm / STEP_CM);
    let rounded = (steps + QUANTISE_BIAS).round_ties_even() as i64;
    (rounded as f32) * STEP_CM
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
        if current + f64::from(STEP_CM) * GROW_STEPS < radius {
            return quantised;
        }
    } else if quantised < current_cm && current - f64::from(STEP_CM) * SHRINK_STEPS > radius {
        return quantised;
    }
    current_cm
}

/// The centre snapped to the texel grid of a `map_size` shadow map: x and y
/// each become `round(c / texel) · texel` with `texel = 2·half_extent / size`.
/// A half extent of zero (or a non-finite centre) leaves the axis alone.
pub fn snapped_centre(centre: Vec3, half_extent_cm: f32, map_size: UVec2) -> Vec3 {
    let mut out = centre;
    for (i, size) in [map_size.x, map_size.y].into_iter().enumerate() {
        let texel = (half_extent_cm * 2.0) / size as f32;
        if texel.abs() <= 0.0 || !texel.is_finite() {
            continue;
        }
        let steps = f64::from(out[i] / texel).round_ties_even();
        if steps.is_finite() {
            out[i] = (steps as f32) * texel;
        }
    }
    out
}

/// The softness the update writes, and the `c'` the render routine's depth
/// offset then uses. `c` is the sun light's own value (0 in the original's
/// default): `(c + 1)·400 / half extent`, floored at [`SOFTNESS_MIN`], in
/// which case `c' = 0.15 / (the unclamped softness)`.
pub fn sun_softness(c: f32, half_extent_cm: f32) -> SunSoftness {
    let raw = (c + 1.0) * (SOFTNESS_CM / half_extent_cm);
    if f64::from(raw) < f64::from(SOFTNESS_MIN) {
        SunSoftness {
            caster: (f64::from(SOFTNESS_MIN) / f64::from(raw)) as f32,
            value: SOFTNESS_MIN,
        }
    } else {
        SunSoftness {
            caster: c,
            value: raw,
        }
    }
}

/// The sun light's filter softness and the offset scale derived from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunSoftness {
    /// The value the depth offset uses (`c'`).
    pub caster: f32,
    /// The filter softness written into the light (`s`).
    pub value: f32,
}

/// The smallest sphere around the cameras' clipped corner sets, for callers
/// that want the fit without the state (tests and tools).
pub fn fitted_sphere(region: &Region) -> Option<Sphere> {
    let corners = region.corners();
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
        assert_eq!(SCENE_MIDDLE_CM.to_bits(), 0x4316_0000);
        assert_eq!(SCENE_HALF_HEIGHT_CM.to_bits(), 0x4422_8000);
        assert_eq!(SOFTNESS_MIN.to_bits(), 0x3e19_999a);
        assert_eq!(SOFTNESS_CM.to_bits(), 0x43c8_0000);
        assert_eq!(GROW_STEPS.to_bits(), 0x3fd3_3333_3333_3333);
        assert_eq!(SHRINK_STEPS.to_bits(), 0x3fb9_9999_9999_999a);
        assert_eq!(QUANTISE_BIAS.to_bits(), 0x3fd9_9999_9999_999a);
    }

    #[test]
    fn the_radius_quantises_to_the_nearest_step_plus_0_4() {
        // 200·round(r/200 + 0.4), ties to even.
        assert_eq!(quantised_half_extent(0.0), 0.0); // 0.4 → 0
        assert_eq!(quantised_half_extent(3000.0), 3000.0); // 15.4 → 15
        assert_eq!(quantised_half_extent(3019.0), 3000.0); // 15.495 → 15
        assert_eq!(quantised_half_extent(3020.0), 3200.0); // 15.500 → 16
        assert_eq!(quantised_half_extent(2819.0), 2800.0); // 14.495 → 14
        assert_eq!(quantised_half_extent(2820.0), 3000.0); // 14.100 + 0.4 → 15
        assert_eq!(quantised_half_extent(2800.0), 2800.0); // 14.4 → 14
        assert_eq!(quantised_half_extent(866.0), 1000.0); // 4.73 → 5
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
    }

    #[test]
    fn the_centre_snaps_to_the_texel_grid() {
        // half 1000 cm over 4096 texels: 0.48828125 cm.
        let map = UVec2::new(4096, 4096);
        let snapped = snapped_centre(v(10.0, -10.0, 500.0), 1000.0, map);
        let texel = 2000.0 / 4096.0;
        assert_eq!(snapped.x, 20.0 * texel);
        assert_eq!(snapped.y, -20.0 * texel);
        assert_eq!(snapped.z, 500.0);
        // A radius of zero leaves the centre alone instead of dividing by 0.
        assert_eq!(snapped_centre(v(1.0, 2.0, 3.0), 0.0, map), v(1.0, 2.0, 3.0));
    }

    #[test]
    fn the_light_basis_is_the_originals() {
        // The sun straight overhead: the fixed frame.
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        assert_eq!(basis.dir, Vec3::NEG_Y);
        assert_eq!(basis.right, Vec3::X);
        assert_eq!(basis.up, Vec3::NEG_Z);
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
    fn the_direction_planes_keep_the_suns_side() {
        let scene = SceneBox::from_bounds(v(-1000.0, -200.0, -500.0), v(3000.0, 400.0, 1500.0));
        // Sun rays travel down and towards −X and +Z.
        let planes = direction_clip_planes(scene, v(-0.5, -1.0, 0.5));
        assert_eq!(planes.len(), 3);
        // x: rays go −X, so keep x ≤ the scene's high edge, 3000.
        assert_eq!(planes[0], ClipPlane::new(Vec3::X, 3000.0));
        // y uses the forced box: 150 ± 650, and rays go down: y ≤ 800.
        assert_eq!(planes[1], ClipPlane::new(Vec3::Y, 800.0));
        // z: rays go +Z, so keep −z ≤ extent − centre = 500, i.e. z ≥ −500.
        assert_eq!(planes[2], ClipPlane::new(Vec3::NEG_Z, 500.0));
        // A zero component adds no plane.
        let planes = direction_clip_planes(scene, v(0.0, -1.0, 0.0));
        assert_eq!(planes.len(), 1);
        assert_eq!(planes[0], ClipPlane::new(Vec3::Y, 800.0));
    }

    #[test]
    fn the_slab_planes_are_the_level_height() {
        assert_eq!(
            slab_clip_planes(0.0, 340.0),
            [
                ClipPlane::new(Vec3::Y, 340.0),
                ClipPlane::new(Vec3::NEG_Y, 0.0),
            ]
        );
    }

    #[test]
    fn a_box_clipped_by_a_middle_plane_keeps_its_corners() {
        // A 1000 cm cube around the origin cut by x ≥ −500 (its own face):
        // the region is unchanged and still has eight corners.
        let mut region = Region::frustum([
            v(-500.0, -500.0, -500.0),
            v(500.0, -500.0, -500.0),
            v(500.0, 500.0, -500.0),
            v(-500.0, 500.0, -500.0),
            v(-500.0, -500.0, 500.0),
            v(500.0, -500.0, 500.0),
            v(500.0, 500.0, 500.0),
            v(-500.0, 500.0, 500.0),
        ]);
        assert_eq!(region.corners().len(), 8);
        region.clip(ClipPlane::new(Vec3::NEG_X, 500.0));
        assert_eq!(region.corners().len(), 8);
        // Cutting through the middle leaves the eight corners of the half.
        region.clip(ClipPlane::new(Vec3::X, 0.0));
        let corners = region.corners();
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
    fn a_fit_of_a_cube_is_its_sphere() {
        // A 1000 cm cube's corners lie on a sphere of radius √3·500.
        let region = Region::frustum([
            v(-500.0, -500.0, -500.0),
            v(500.0, -500.0, -500.0),
            v(500.0, 500.0, -500.0),
            v(-500.0, 500.0, -500.0),
            v(-500.0, -500.0, 500.0),
            v(500.0, -500.0, 500.0),
            v(500.0, 500.0, 500.0),
            v(-500.0, 500.0, 500.0),
        ]);
        let sphere = fitted_sphere(&region).unwrap();
        assert!(sphere.centre.length() < 1e-4);
        let radius = 3f32.sqrt() * 500.0;
        assert!((sphere.radius - radius).abs() < 1e-3);
        // The steps: 866.025/200 + 0.4 = 4.73 → 5 steps = 1000 cm.
        assert_eq!(quantised_half_extent(sphere.radius), 1000.0);
    }

    #[test]
    fn fewer_than_two_corners_fit_nothing() {
        let empty = Region::default();
        let mut fit = SunFit::default();
        assert!(!fit.fit(
            &empty,
            &LightBasis::from_towards_sun(Vec3::Y),
            UVec2::splat(4096)
        ));
        assert_eq!(fit.half_extent_cm, INITIAL_HALF_EXTENT_CM);
        // A region whose corners all merge into one point.
        let point = Region::from_faces(vec![vec![v(10.0, 10.0, 10.0); 4]]);
        assert!(!fit.fit(
            &point,
            &LightBasis::from_towards_sun(Vec3::Y),
            UVec2::splat(4096)
        ));
    }

    #[test]
    fn a_region_clipped_by_the_suns_planes_fits_its_sphere() {
        // A 2000 cm cube at the origin and a sun straight up: the forced
        // vertical box keeps y ≤ 800, so four corners stay at y = −1000 and
        // four are cut in at y = 800.
        let mut region = Region::frustum([
            v(-1000.0, -1000.0, -1000.0),
            v(1000.0, -1000.0, -1000.0),
            v(1000.0, 1000.0, -1000.0),
            v(-1000.0, 1000.0, -1000.0),
            v(-1000.0, -1000.0, 1000.0),
            v(1000.0, -1000.0, 1000.0),
            v(1000.0, 1000.0, 1000.0),
            v(-1000.0, 1000.0, 1000.0),
        ]);
        let scene = SceneBox {
            centre: v(0.0, 5000.0, 0.0),
            extent: Vec3::splat(100000.0),
        };
        for plane in direction_clip_planes(scene, Vec3::NEG_Y) {
            region.clip(plane);
        }
        let sphere = fitted_sphere(&region).unwrap();
        // Centre (0, −100, 0) cm; r² = 1000² + 900² + 1000².
        assert!((sphere.centre - v(0.0, -100.0, 0.0)).length() < 1e-2);
        assert!((sphere.radius - 2.81f32.sqrt() * 1000.0).abs() < 1e-2);
        // In the light's frame (right +X, up −Z, dir −Y) the centre is
        // (0, 0, 100); 1676.3/200 + 0.4 = 8.78 → 9 steps.
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let mut fit = SunFit::default();
        assert!(fit.fit(&region, &basis, UVec2::splat(4096)));
        assert_eq!(fit.half_extent_cm, 1800.0);
        assert_eq!(fit.centre_cm, v(0.0, 0.0, 100.0));
    }

    #[test]
    fn a_region_wholly_outside_a_plane_fits_nothing() {
        let mut region = Region::frustum([
            v(-500.0, 900.0, -500.0),
            v(500.0, 900.0, -500.0),
            v(500.0, 1500.0, -500.0),
            v(-500.0, 1500.0, -500.0),
            v(-500.0, 900.0, 500.0),
            v(500.0, 900.0, 500.0),
            v(500.0, 1500.0, 500.0),
            v(-500.0, 1500.0, 500.0),
        ]);
        region.clip(ClipPlane::new(Vec3::Y, 800.0));
        assert!(region.is_empty());
        assert!(region.corners().is_empty());
        let mut fit = SunFit::default();
        assert!(!fit.fit(
            &region,
            &LightBasis::from_towards_sun(Vec3::Y),
            UVec2::splat(4096)
        ));
        assert_eq!(fit.half_extent_cm, INITIAL_HALF_EXTENT_CM);
    }

    #[test]
    fn the_fit_quantises_and_snaps() {
        let region = Region::frustum([
            v(-500.0, -500.0, -500.0),
            v(500.0, -500.0, -500.0),
            v(500.0, 500.0, -500.0),
            v(-500.0, 500.0, -500.0),
            v(-500.0, -500.0, 500.0),
            v(500.0, -500.0, 500.0),
            v(500.0, 500.0, 500.0),
            v(-500.0, 500.0, 500.0),
        ]);
        let basis = LightBasis::from_towards_sun(Vec3::Y);
        let mut fit = SunFit::default();
        assert!(fit.fit(&region, &basis, UVec2::splat(4096)));
        // (866.025)/200 + 0.4 = 4.73 → 5 steps.
        assert_eq!(fit.half_extent_cm, 1000.0);
        // The centre is the origin: already on the grid.
        assert_eq!(fit.centre_cm, Vec3::ZERO);
    }

    #[test]
    fn the_matrices_cover_the_fitted_box() {
        let fit = SunFit {
            half_extent_cm: 1000.0,
            centre_cm: Vec3::ZERO,
            softness: 0.0,
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
}
