//! The navigation controller's free-camera drags and its zoom handlers: the
//! absolute-position orbit, the vertical zoom slide, the stepped wheel zoom,
//! the zoom's pitch offset and the editor's grab-drag. They are the
//! arithmetic of the unit's routines with the controller's fields as
//! arguments; the chase camera's own spring (`crate::player`) keeps its
//! state. Rules: `docs/PLAYER.md` (Camera).
//!
//! The original's handlers take a controller object, a GUI mouse event and a
//! handler slot. Exarust has neither the editor viewport nor its callbacks,
//! so each routine is a pure function here, with tests, and the chase camera
//! reuses the parts it can ([`zoom_detents`] for the wheel).

use bevy::math::Vec3;
use exa_physics::engine_math::{acos, asin, exp, ln, max, min, rotate_about, round_to_i64, sign};

use crate::nav_camera::{camera_axis, planar_yaw};
use crate::thaum::engine::normalize;

/// One wheel unit of the original's zoom handlers: the single nearest 1/120
/// (120 units to a notch).
pub const WHEEL_UNIT: f32 = 0.008_333_334;
/// The wheel units in one notch of the mouse wheel: a line in Bevy's scroll
/// events.
pub const WHEEL_UNITS_PER_NOTCH: f32 = 120.0;
/// A stepped zoom's stop as a share of its range (a fifth: six stops).
pub const ZOOM_STOP_SHARE: f32 = 0.2;
/// The stopped zoom's top level: the stops are 0–5 above the least zoom.
pub const ZOOM_TOP_LEVEL: f32 = 5.0;
/// The vertical zoom slide's pixel scale (zoom per pixel).
pub const ZOOM_SLIDE_PER_PIXEL: f32 = 0.001;
/// The editor's grab drag turns this share of the orbit rate per pixel (a
/// half).
pub const GRAB_TURN_SHARE: f32 = 0.5;

/// The stepped camera's zoom after a wheel of `units` (120 to a detent), as
/// the unit's wheel routine moves it when the camera is stepped: the current
/// zoom is read as a stop level (`(zoom − min) / stop`, rounded to the
/// nearest, ties to even), the move `units · 0.008333334` is subtracted and
/// held between 0 and 5 by the processor's max-then-min (a NaN passes), and
/// the level is turned back into a zoom over the stop. A level that is not an
/// integer is kept, so the result is not always exactly on a stop.
pub fn zoom_detents(zoom: f32, units: i32, low: f32, high: f32) -> f32 {
    let stop = (high - low) * ZOOM_STOP_SHARE;
    let level = round_to_i64((zoom - low) / stop) as f32;
    let level = max(level - units as f32 * WHEEL_UNIT, 0.0);
    let level = min(level, ZOOM_TOP_LEVEL);
    level * stop + low
}

/// The zoom after a slide of the pointer `dy` pixels from where the slide
/// began: `clamp(dy · 0.001 + base, 0, 1)² · (max − min) + min`, the clamp by
/// the processor's max-then-min (a NaN passes), the square taken first and
/// then multiplied into the range, all in single precision.
pub fn zoom_slide(dy: f32, base: f32, low: f32, high: f32) -> f32 {
    let t = max(0.0, dy * ZOOM_SLIDE_PER_PIXEL + base);
    let t = min(1.0, t);
    (high - low) * (t * t) + low
}

/// The pitch offset a zoom asks for, when the controller's amplitude is not
/// zero: `exp(3·ln(t)) · amplitude` with `t = 1 − (zoom − min) / (max − min)`,
/// the engine's fast logarithm and exponential and every step single
/// precision. A NaN amplitude is not zero and passes the test; a zero one
/// gives `None` (the routine leaves the pitch alone).
pub fn zoom_pitch_offset(zoom: f32, low: f32, high: f32, amplitude: f32) -> Option<f32> {
    if amplitude == 0.0 {
        return None;
    }
    let t = 1.0 - (zoom - low) / (high - low);
    Some(exp(ln(t) * 3.0) * amplitude)
}

/// The camera pitch after the zoom's offset is applied: the offset holds only
/// while it is steeper than the present pitch (`−pitch < offset`), then the
/// pitch becomes `−offset`. A NaN offset leaves the pitch alone.
pub fn pitch_with_offset(pitch: f32, offset: f32) -> f32 {
    if -pitch < offset { -offset } else { pitch }
}

/// The state an absolute-position drag snapshots, with the pixel bounds the
/// original hands the GUI so the pointer cannot leave the pitch's range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreeDrag {
    /// The yaw the drag started from (the controller's shown yaw).
    pub base_yaw: f32,
    /// The pitch the drag started from: minus the engine's arc sine of the
    /// view's backward vector's height.
    pub base_pitch: f32,
    /// The yaw and pitch rates per pixel and the preset's sensitivity.
    pub yaw_rate: f32,
    pub pitch_rate: f32,
    pub sensitivity: f32,
    /// The bounds the pitch's negation is held between.
    pub low: f32,
    pub high: f32,
    /// The pointer's vertical pixel range, inclusive, that keeps the pitch
    /// within the bounds: `y ± round((bound + base_pitch) / (rate · sens))`.
    pub pixel_low: i32,
    pub pixel_high: i32,
}

/// Starts an absolute-position drag at pointer `y`: the yaw is kept as the
/// base, the pitch is `−asin(back.y)` (the view's backward vector in the
/// camera's frame), and the pointer's allowed vertical range is built from
/// the two pitch bounds. Each bound is added to the base pitch, divided by
/// the pitch rate times the sensitivity and rounded to the nearest (ties to
/// even), then offset from the pointer.
#[allow(clippy::too_many_arguments)]
pub fn free_drag_start(
    yaw: f32,
    back_y: f32,
    yaw_rate: f32,
    pitch_rate: f32,
    sensitivity: f32,
    low: f32,
    high: f32,
    pointer_y: i32,
) -> FreeDrag {
    let base_pitch = -asin(back_y);
    let divisor = pitch_rate * sensitivity;
    FreeDrag {
        base_yaw: yaw,
        base_pitch,
        yaw_rate,
        pitch_rate,
        sensitivity,
        low,
        high,
        // The 64-bit sum is stored as a 32-bit word: a wrapping add.
        pixel_low: pointer_y.wrapping_add(round_to_i64((low + base_pitch) / divisor) as i32),
        pixel_high: pointer_y.wrapping_add(round_to_i64((high + base_pitch) / divisor) as i32),
    }
}

impl FreeDrag {
    /// The yaw and pitch at a pointer that has moved `(dx, dy)` pixels from
    /// the drag's start: `base − rate · sens · delta` for the yaw, and the
    /// same for the pitch, whose negation is then held between [`Self::low`]
    /// and [`Self::high`] (the steep bound first, a NaN passes both).
    pub fn at(&self, dx: f32, dy: f32) -> (f32, f32) {
        let yaw = self.base_yaw - self.yaw_rate * self.sensitivity * dx;
        let mut pitch = self.base_pitch - self.pitch_rate * self.sensitivity * dy;
        if -pitch < self.low {
            pitch = -self.low;
        } else if -pitch > self.high {
            pitch = -self.high;
        }
        (yaw, pitch)
    }
}

/// The yaw a released absolute drag leaves, when the drag handler's close
/// message arrives: if the yaw is more than about an eighth of a turn from
/// the shown yaw (the absolute distance over a right angle rounds above
/// zero), it snaps a quarter turn from the shown yaw in the direction it
/// moved; otherwise it stays. The turn is the single `π/2`. The original
/// tests only the low 32 bits of the rounded count, so a NaN (whose count
/// is the integer indefinite) or a count of a multiple of 2³² stays.
pub fn free_drag_release(yaw: f32, shown_yaw: f32) -> f32 {
    const QUARTER: f32 = std::f32::consts::FRAC_PI_2;
    let quarters = round_to_i64((yaw - shown_yaw).abs() / QUARTER) as i32;
    if quarters > 0 {
        sign(yaw - shown_yaw) * QUARTER + shown_yaw
    } else {
        yaw
    }
}

/// The editor's grab drag: the clicked point, the distance the camera keeps
/// from it along the view and the yaw the drag started from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrabDrag {
    /// The clicked point at the looked-at point's height.
    pub grab: Vec3,
    /// The camera's distance from the point (the node's radius plus the
    /// zoom).
    pub reach: f32,
    /// The yaw the drag started from.
    pub base_yaw: f32,
    /// The drag's height base, `dy · pan + base` each move.
    pub base_height: f32,
}

/// Starts an editor grab drag on `node` (its world position and the x half
/// extent of its box) with the camera's looked-at `target` and its `zoom`:
/// the grabbed point takes the node's place on the ground but the target's
/// height, the camera keeps `half_extent + zoom` from it, and the base yaw is
/// the engine's planar yaw of the target less the grabbed point, normalised
/// first by the engine's normalise as the original does.
pub fn grab_start(node: Vec3, half_extent: f32, target: Vec3, zoom: f32) -> GrabDrag {
    let grab = Vec3::new(node.x, target.y, node.z);
    GrabDrag {
        grab,
        reach: half_extent + zoom,
        base_yaw: planar_yaw(normalize(target - grab)),
        base_height: target.y,
    }
}

impl GrabDrag {
    /// The drag after a pointer move of `(dx, dy)` from the press, with the
    /// viewport's width and aspect: the yaw is `base − dx · rate · 0.5`; the
    /// height is `dy · pan + base`, `pan` being the zoom over half the
    /// viewport's world width; the looked-at point is the grabbed point plus
    /// the view's backward vector at the new yaw (pitch zero) times the
    /// reach. The grabbed point is raised to the new height before the
    /// looked-at point is built from it, as the original writes the height
    /// first. Returns the yaw, the height the camera keeps for the point and
    /// the new looked-at point.
    pub fn moved(
        &self,
        dx: f32,
        dy: f32,
        rate: f32,
        viewport_width: f32,
        aspect: f32,
        zoom: f32,
    ) -> (f32, f32, Vec3) {
        let yaw = self.base_yaw - dx * rate * GRAB_TURN_SHARE;
        let pan = zoom / (viewport_width * aspect * 0.5);
        let height = dy * pan + self.base_height;
        let grab = Vec3::new(self.grab.x, height, self.grab.z);
        let target = grab + camera_axis(0.0, yaw) * self.reach;
        (yaw, height, target)
    }
}

/// The editor's other grab target, from a mesh hit: the camera point is
/// turned about the hit's face normal until it looks along it (the engine's
/// arc cosine of their dot, then a rotation about their cross product, both
/// with the fast trig), and the looked-at point is the hit point that many
/// zoom units along the turned view. A view already along the normal (a zero
/// angle) is left as it is.
///
/// [guess: the original rotates about the unnormalised cross product, as its
/// rotate call receives it; the axis's length is not corrected here]
pub fn grab_target(click: Vec3, camera: Vec3, normal: Vec3, zoom: f32) -> Vec3 {
    let view = normalize(camera - click);
    // The original's dot sums z·n.z + (y·n.y + x·n.x).
    let dot = view.z * normal.z + (view.y * normal.y + view.x * normal.x);
    let angle = acos(dot);
    let view = if angle > 0.0 {
        normalize(rotate_about(view.cross(normal), view, angle))
    } else {
        view
    };
    click + view * zoom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stepped_wheel_zoom_reads_and_writes_stops() {
        // One notch from the bottom: level 0 − 1 held at 0.
        assert_eq!(zoom_detents(800.0, 120, 800.0, 2000.0), 800.0);
        // From the third stop, one notch (120 units, exactly one level in
        // the original's single) falls to the second.
        assert_eq!(zoom_detents(1280.0, 120, 800.0, 2000.0), 1040.0);
        // One notch from the second stop falls to the first.
        assert_eq!(zoom_detents(1040.0, 120, 800.0, 2000.0), 800.0);
        // Ties to even: 900 is half a stop from 800, so the level is 0.
        assert_eq!(zoom_detents(900.0, 0, 800.0, 2000.0), 800.0);
        assert_eq!(zoom_detents(920.0, 0, 800.0, 2000.0), 800.0);
        assert_eq!(zoom_detents(940.0, 0, 800.0, 2000.0), 1040.0);
        // Outward from the top stays at five stops.
        assert_eq!(zoom_detents(2000.0, -120, 800.0, 2000.0), 2000.0);
        assert_eq!(zoom_detents(2000.0, 0, 800.0, 2000.0), 2000.0);
        // The current zoom is always read as a level, even with no wheel:
        // 1000 lies 5/6 of the way to the next stop, 1160 is the tie that
        // rounds to the even level 2.
        assert_eq!(zoom_detents(1000.0, 0, 800.0, 2000.0), 1040.0);
        assert_eq!(zoom_detents(1160.0, 0, 800.0, 2000.0), 1280.0);
        // A NaN zoom's level is the integer indefinite, which the max holds
        // at zero with the second operand on NaN: the least zoom.
        assert_eq!(zoom_detents(f32::NAN, 0, 800.0, 2000.0), 800.0);
    }

    #[test]
    fn a_vertical_slide_squares_its_position() {
        // At the base the zoom is the least; halfway, a quarter of the range.
        assert_eq!(zoom_slide(0.0, 0.0, 800.0, 2000.0), 800.0);
        assert_eq!(zoom_slide(500.0, 0.0, 800.0, 2000.0), 1100.0);
        assert_eq!(zoom_slide(1000.0, 0.0, 800.0, 2000.0), 2000.0);
        // A base offsets the position, and the position is held to 0–1.
        assert_eq!(zoom_slide(0.0, 0.25, 800.0, 2000.0), 875.0);
        assert_eq!(zoom_slide(-9000.0, 0.0, 800.0, 2000.0), 800.0);
        assert_eq!(zoom_slide(9000.0, 0.0, 800.0, 2000.0), 2000.0);
        assert!(zoom_slide(f32::NAN, 0.0, 800.0, 2000.0).is_nan());
    }

    #[test]
    fn the_zooms_pitch_offset_is_a_cubed_fall() {
        // A zoom at `min` gives t = 1 and the full amplitude; at `max` zero.
        assert_eq!(
            zoom_pitch_offset(800.0, 800.0, 2000.0, 0.3),
            Some(exp(ln(1.0) * 3.0) * 0.3)
        );
        assert_eq!(zoom_pitch_offset(2000.0, 800.0, 2000.0, 0.3), Some(0.0));
        // Halfway t = 0.5: an eighth of the amplitude, through the fast
        // logarithm and exponential (near, not equal, 0.0375).
        let half = zoom_pitch_offset(1400.0, 800.0, 2000.0, 0.3).unwrap();
        assert!((half - 0.0375).abs() < 1e-4, "{half}");
        // A zero amplitude leaves the pitch alone; a NaN one does not.
        assert_eq!(zoom_pitch_offset(1400.0, 800.0, 2000.0, 0.0), None);
        assert!(
            zoom_pitch_offset(1400.0, 800.0, 2000.0, f32::NAN)
                .unwrap()
                .is_nan()
        );
        // The offset only ever steepens the pitch.
        assert_eq!(pitch_with_offset(-0.5, 0.8), -0.8);
        assert_eq!(pitch_with_offset(-0.9, 0.8), -0.9);
        assert_eq!(pitch_with_offset(-0.5, f32::NAN), -0.5);
    }

    #[test]
    fn an_absolute_drag_reads_deltas_from_its_start() {
        let drag = free_drag_start(0.4, 0.5, 0.0025, 0.0025, 2.0, -1.5, 1.5, 400);
        // The back vector's height gives −asin(0.5) = −π/6.
        assert!((drag.base_pitch + asin(0.5)).abs() < 1e-6);
        assert_eq!(drag.base_yaw, 0.4);
        // A zero move stays at the base.
        assert_eq!(drag.at(0.0, 0.0), (0.4, drag.base_pitch));
        // 100 px right and 100 px down: yaw and pitch both 0.5 lower.
        let (yaw, pitch) = drag.at(100.0, 100.0);
        assert!((yaw - (0.4 - 0.5)).abs() < 1e-6);
        assert!((pitch - (drag.base_pitch - 0.5)).abs() < 1e-6);
        // The pitch is held so its negation stays between −1.5 and 1.5.
        let (_, pitch) = drag.at(0.0, 10_000.0);
        assert_eq!(pitch, -1.5);
        let (_, pitch) = drag.at(0.0, -10_000.0);
        assert_eq!(pitch, 1.5);
        // The pointer's allowed range is the pixel where each bound lands.
        let low = 400 + round_to_i64((-1.5 + drag.base_pitch) / (0.0025 * 2.0)) as i32;
        let high = 400 + round_to_i64((1.5 + drag.base_pitch) / (0.0025 * 2.0)) as i32;
        assert_eq!((drag.pixel_low, drag.pixel_high), (low, high));
    }

    #[test]
    fn a_released_drag_snaps_a_quarter_turn() {
        // Less than an eighth of a turn from the shown yaw: kept.
        assert_eq!(free_drag_release(0.5, 0.0), 0.5);
        // An eighth or more: a quarter turn in the direction moved.
        assert_eq!(free_drag_release(0.9, 0.0), std::f32::consts::FRAC_PI_2);
        assert_eq!(free_drag_release(-0.9, 0.0), -std::f32::consts::FRAC_PI_2);
        // The shown yaw is the centre, and a NaN stays.
        assert_eq!(
            free_drag_release(1.0 + std::f32::consts::FRAC_PI_2, 1.0),
            1.0 + std::f32::consts::FRAC_PI_2
        );
        assert!(free_drag_release(f32::NAN, 0.0).is_nan());
    }

    #[test]
    fn a_grabbed_node_is_dragged_around_the_camera() {
        let drag = grab_start(Vec3::new(2.0, 5.0, 0.0), 1.0, Vec3::ZERO, 10.0);
        // The grabbed point takes the node's x and z and the target's y.
        assert_eq!(drag.grab, Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(drag.reach, 11.0);
        // Looking from the grabbed point to the target (−X): yaw −π/2.
        assert!((drag.base_yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        // No move: the camera sits `reach` along +X from the grabbed point.
        let (yaw, height, target) = drag.moved(0.0, 0.0, 0.0025, 1600.0, 1.6, 10.0);
        assert_eq!(yaw, drag.base_yaw);
        assert_eq!(height, 0.0);
        let back = Vec3::new(yaw.sin(), 0.0, yaw.cos()) * 11.0;
        assert!((target - (drag.grab + back)).length() < 1e-4);
        // 100 px right turns the yaw by 100 · 0.0025 · 0.5 = 0.125.
        let (turned, _, _) = drag.moved(100.0, 0.0, 0.0025, 1600.0, 1.6, 10.0);
        assert!((turned - (drag.base_yaw - 0.125)).abs() < 1e-6);
        // 100 px down raises the point by 100 · zoom / (width·aspect/2) =
        // 100 · 10 / 1280.
        let (_, height, target) = drag.moved(0.0, 100.0, 0.0025, 1600.0, 1.6, 10.0);
        assert!((height - 100.0 * 10.0 / 1280.0).abs() < 1e-6);
        // The looked-at point rides at the raised height.
        assert_eq!(target.y, height);
    }

    #[test]
    fn a_mesh_grab_turns_the_view_onto_the_face() {
        // The view from (0, 0, 10) to the origin already looks along −Z:
        // with that normal the angle is zero and the point is 5 m ahead.
        let target = grab_target(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), -Vec3::Z, 5.0);
        assert!((target - Vec3::new(0.0, 0.0, -5.0)).length() < 1e-3);
        // A face normal of +Z is a half turn away: the cross product is
        // zero, so the engine's rotation takes the cosine, −1, and turns
        // the view to +Z.
        let target = grab_target(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), Vec3::Z, 5.0);
        assert!((target - Vec3::new(0.0, 0.0, 5.0)).length() < 1e-3);
    }
}
