//! The original's camera navigation routines: where the view sits, the yaw
//! that turns it behind a body, the orbit drag and its pitch limits, the
//! zoom steps, the frame clock, a kick, the preset record with its editor
//! target scan and handler set, the viewport attachment, and the unit's
//! jittered offset table. The chase camera in [`crate::player`] uses these;
//! the free-camera drags and zoom handlers are [`crate::nav_drag`]. The
//! rules they reproduce are in `docs/PLAYER.md` (Camera).
//!
//! The original keeps its camera state in one controller and its arithmetic
//! in a navigation unit. This module is that unit: plain functions, no
//! state, so the chase camera's fields keep the controller's job.

use bevy::math::Vec3;
use exa_ai::search::quick_asin;

use crate::thaum::engine::{length, normalize, sin_cos};

/// Orbit and camera-look sensitivity on a screen 1920 pixels wide (rad per
/// pixel); [`orbit_per_pixel`] gives it for any width.
pub const ORBIT_PER_PIXEL: f32 = 0.0025;
/// The orbit's turn across the whole screen width (rad).
pub const ORBIT_PER_SCREEN: f64 = 4.8;
/// The camera's tick (s) and how many a frame may run.
pub const TICK: f32 = 0.017;
pub const MAX_TICKS: u32 = 9;
/// The pitch floor under zoom 1000, the added steepness per zoom unit and
/// the zoom where the added part starts (rad, rad per unit, original zoom).
pub const PITCH_BASE: f32 = 0.8;
pub const PITCH_PER_ZOOM: f64 = 0.000125;
pub const PITCH_ZOOM_BASE: f32 = 1000.0;
/// The steepest pitch a vertical drag reaches (rad below the horizon).
pub const PITCH_CAP: f32 = 1.5;
/// The original's "a distance this small is not a distance": 2^-23.
pub const DISTANCE_EPSILON: f32 = 1.192_089_982_282_595_9e-7;
/// A free zoom moves by this share of the current value per wheel unit
/// (the original's double 0.002).
pub const ZOOM_RATE_PER_UNIT: f64 = 0.002;
/// A free zoom's smallest step: this share of the whole range.
pub const ZOOM_STEP_SHARE: f32 = 0.005;
/// A kick larger than this (squared) is normalised to its size.
const KICK_EPSILON: f32 = 1.192_089_982_282_595_9e-7;
/// A kick direction shorter than this (squared, compared in double
/// precision) is replaced by the camera's right axis.
const KICK_DEGENERATE: f64 = 0.1;

/// The orbit sensitivity for a screen `width` pixels wide: 4.8 rad over the
/// width, divided in double precision and stored as a single, as the
/// original's preset sets it.
pub fn orbit_per_pixel(width: f32) -> f32 {
    (ORBIT_PER_SCREEN / f64::from(width)) as f32
}

/// The engine's yaw of a vector about the vertical, as the original's
/// planar-angle routine takes it: zero when the horizontal length squared
/// (`z² + x²`) is under 1e-8, otherwise the engine's arc cosine of `z` over
/// that length (π at or below −1, zero above 1), negated for a negative x.
/// A NaN length or cosine falls through to the arc cosine, as the
/// unordered compares do.
pub fn planar_yaw(v: Vec3) -> f32 {
    let length_squared = v.z * v.z + v.x * v.x;
    if length_squared < 1e-8 {
        return 0.0;
    }
    let cosine = v.z / length_squared.sqrt();
    let angle = if cosine <= -1.0 {
        std::f32::consts::PI
    } else if cosine > 1.0 {
        0.0
    } else {
        fast_acos(cosine)
    };
    if v.x < 0.0 { -angle } else { angle }
}

/// The engine's arc cosine: `π − (π/2 + 0.2075·x + 0.0535·x²)·√(1 + x)`
/// below zero (the subtraction in double precision) and
/// `(π/2 − 0.2075·x + 0.0535·x²)·√(1 − x)` from zero up, both in single
/// precision with the original's bit-exact coefficients. A NaN takes the
/// second form.
pub(crate) fn fast_acos(x: f32) -> f32 {
    const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
    const LINEAR: f32 = f32::from_bits(0x3e54_7ae1);
    const SQUARE: f32 = f32::from_bits(0x3d5b_22d1);
    if x < 0.0 {
        let poly = x * x * SQUARE + (x * LINEAR + HALF_PI);
        (std::f64::consts::PI - f64::from(poly * (1.0 + x).sqrt())) as f32
    } else {
        let poly = x * x * SQUARE + (HALF_PI - x * LINEAR);
        poly * (1.0 - x).sqrt()
    }
}

/// The yaw a camera turns to so that it looks along a direction whose yaw
/// is `target_yaw`, as the original's align routine sets it: the target is
/// kept within half a turn of the camera's present yaw (`camera_yaw`, the
/// yaw of its backward vector), `target + 2π` when the camera lies more
/// than π beyond it and `target − 2π` when it lies more than π short, so
/// the orbit spring turns the short way. The differences are single
/// precision, compared and turned in double precision, as the original
/// does. A NaN yaw is left alone.
pub fn align_behind(target_yaw: f32, camera_yaw: f32) -> f32 {
    if f64::from(camera_yaw - target_yaw) > std::f64::consts::PI {
        (f64::from(target_yaw) + std::f64::consts::TAU) as f32
    } else if f64::from(target_yaw - camera_yaw) > std::f64::consts::PI {
        (f64::from(target_yaw) - std::f64::consts::TAU) as f32
    } else {
        target_yaw
    }
}

/// The pitch the zoom asks for: 0.8 rad below the horizon under zoom 1000,
/// then 0.000125 rad steeper per unit. The zoom less 1000 is a single, the
/// product and the sum with 0.8 are doubles, as the original's are; an
/// unordered compare takes the steep branch too, so a NaN zoom gives a NaN
/// floor.
pub fn pitch_floor(zoom: f32) -> f32 {
    if zoom < PITCH_ZOOM_BASE {
        PITCH_BASE
    } else {
        (f64::from(zoom - PITCH_ZOOM_BASE) * PITCH_PER_ZOOM + 0.8) as f32
    }
}

/// The pitch the camera holds: the asked-for pitch, or the zoom's floor
/// when the floor is steeper. The original keeps the asked-for pitch apart,
/// so it comes back when the floor drops again; a NaN floor takes the asked
/// pitch, as its unordered compare does.
pub fn pitch_held(asked: f32, floor: f32) -> f32 {
    if floor > asked { floor } else { asked }
}

/// Holds a pitch between the floor `low` and the cap `high`, with the
/// original's branch order: below the floor takes the floor, above the cap
/// takes the cap, and a NaN stays as it is.
pub fn clamp_pitch(pitch: f32, low: f32, high: f32) -> f32 {
    if low > pitch {
        low
    } else if pitch > high {
        high
    } else {
        pitch
    }
}

/// The yaw after an orbit drag of `dx` pixels at `per_pixel` radians each:
/// the original subtracts the drag's angle.
pub fn orbit_yaw(yaw: f32, dx: f32, per_pixel: f32) -> f32 {
    yaw - per_pixel * dx
}

/// The pitch after an orbit drag of `dy` pixels at `per_pixel` radians
/// each, held between the floor and the cap. The original first stores the
/// raised value, clamps it, then takes the floor once more.
pub fn orbit_pitch(pitch: f32, dy: f32, per_pixel: f32, low: f32, high: f32) -> f32 {
    let raised = pitch + per_pixel * dy;
    let held = if raised < low {
        low
    } else if raised > high {
        high
    } else {
        raised
    };
    if low > held { low } else { held }
}

/// A free zoom after a wheel turn of `wheel` units (120 to a notch): 0.2 %
/// of the current value per unit (24 % a notch), worked in double precision
/// and stored as a single, but never less than 0.5 % of the whole range,
/// subtracted from the zoom and held in `[min, max]` by the processor's
/// max-then-min (a NaN passes through). The original scales the sign of a
/// step that is too small, so a zero step stays zero.
pub fn zoom_steps(zoom: f32, wheel: i32, min: f32, max: f32) -> f32 {
    let mut delta = (f64::from(wheel) * (f64::from(zoom) * ZOOM_RATE_PER_UNIT)) as f32;
    let least = (max - min) * ZOOM_STEP_SHARE;
    if least > delta.abs() {
        delta = least * sign_or_zero(delta);
    }
    let raised = zoom - delta;
    let held = if min > raised { min } else { raised };
    if max < held { max } else { held }
}

/// The sign of a value as the original's bit trick gives it: −1, 0 or 1. A
/// NaN is not zero and keeps its sign bit.
fn sign_or_zero(value: f32) -> f32 {
    if value == 0.0 {
        0.0
    } else if value.is_sign_negative() {
        -1.0
    } else {
        1.0
    }
}

/// The target height limits of a camera placement: the original clamps the
/// looked-at point to these.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeightLimits {
    pub min: f32,
    pub max: f32,
}

impl HeightLimits {
    /// The height of `point` after the original's two clamps (a NaN is
    /// left alone, an unordered compare taking neither branch).
    pub fn apply(&self, point: Vec3) -> Vec3 {
        let mut point = point;
        if point.y < self.min {
            point.y = self.min;
        }
        if point.y > self.max {
            point.y = self.max;
        }
        point
    }
}

/// Where a camera looks from and at, as setting its target builds it: the
/// looked-at point (its height clamped), the backward unit vector from it
/// to the eye, the distance and the yaw and pitch of that vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    pub at: Vec3,
    pub back: Vec3,
    pub distance: f32,
    pub yaw: f32,
    /// The controller's pitch: minus the engine's arc sine of the backward
    /// vector's height, so negative with the eye above the point.
    pub pitch: f32,
}

/// Places the camera at `at` (height-clamped) looking along `eye_or_back`.
/// With a `distance` of at least 2⁻²³ (or a NaN one, as the unordered
/// compare falls) the second point is taken as the backward vector, as it
/// is. Without one it is the eye: the backward vector runs to it from the
/// point *before* the height clamp (the original subtracts its unclamped
/// copy), the distance is its length (x² + z², then y²) and the vector is
/// scaled by the reciprocal of that length, with no guard for a zero
/// length. The yaw is the engine's planar yaw of the vector
/// ([`planar_yaw`]) and the pitch minus the engine's arc sine of its height.
pub fn place(
    at: Vec3,
    eye_or_back: Vec3,
    distance: Option<f32>,
    limits: HeightLimits,
) -> Placement {
    let clamped = limits.apply(at);
    let (back, distance) = match distance {
        Some(distance) if distance.is_nan() || distance >= DISTANCE_EPSILON => {
            (eye_or_back, distance)
        }
        _ => {
            let vector = eye_or_back - at;
            let distance =
                ((vector.x * vector.x + vector.z * vector.z) + vector.y * vector.y).sqrt();
            (vector * (1.0 / distance), distance)
        }
    };
    Placement {
        at: clamped,
        back,
        distance,
        yaw: planar_yaw(back),
        pitch: -quick_asin(back.y),
    }
}

/// The zoom a stepped camera starts from after a placement: the distance,
/// raised to `min` (10 by default) and, on the stepped camera, lowered to
/// `max` (the zoom range's top).
pub fn clamp_distance(distance: f32, min: f32, max: f32) -> f32 {
    if distance < min {
        min
    } else if distance > max {
        max
    } else {
        distance
    }
}

/// Pulls a placement's pitch below the horizon into the limits the stepped
/// camera keeps: at least the held pitch `floor`, at most `cap`. The
/// original stores the negated value; a NaN takes neither side.
pub fn clamp_fixed_pitch(pitch: f32, floor: f32, cap: f32) -> f32 {
    if floor > pitch {
        floor
    } else if pitch > cap {
        cap
    } else {
        pitch
    }
}

/// The stepped camera's wall-avoidance tilt per step (rad).
pub const WALL_TILT_STEP: f32 = 0.15;
/// The most steps the wall avoidance climbs.
pub const WALL_TILT_MAX: i32 = 4;
/// The highest the camera's axis or a wall probe may look from (the
/// controller's pitch, negative with the eye above the point; the compare
/// is in double precision against −1.54, the stored value its single).
pub const WALL_PITCH_FLOOR: f32 = -1.54;
/// A wall probe looks this much higher than the camera (rad, double).
pub const WALL_PROBE_LIFT: f64 = 0.05;
/// The probe's radius (cm): 30 for the probe that may climb, 20 for the
/// one that may come down.
pub const PROBE_RADIUS_CLIMB: f32 = 30.0;
pub const PROBE_RADIUS_DESCEND: f32 = 20.0;
/// The probe starts this far (cm) below the followed point.
pub const PROBE_DROP: f32 = 50.0;
/// The share of a rising tilt change that is taken (double).
const TILT_RISE: f64 = 0.35;
/// A tilt change is held to this size per tick (double).
const TILT_STEP_LIMIT: f64 = 0.005;
/// The tilt keeps this share of its motion per tick (double).
const TILT_CARRY: f64 = 0.96;

/// The wall probe's length (cm) from the camera's extent (the stepped
/// camera's eye distance, 2000 cm in the game): `extent · 2 − 150`, in
/// single precision.
pub fn probe_length(extent: f32) -> f32 {
    extent * 2.0 - 150.0
}

/// One wall probe's set-up, as the camera's per-tick probe makes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallProbe {
    /// The pitch the probe looks along (the controller's convention).
    pub pitch: f32,
    /// The probe's radius (cm).
    pub radius: f32,
    /// The probe's flag after the set-up: set when this probe tests the
    /// present step (a hit climbs), clear when it tests the step below (a
    /// miss comes down).
    pub climb: bool,
}

/// Sets a wall probe up from the controller's `pitch` (negative with the
/// eye above the point), the step count and the flag the last probe left.
/// With steps taken and the flag set, the probe tests one step below and
/// clears the flag; otherwise it tests the present step and sets it. The
/// pitch is `(pitch − 0.05) − level · 0.15`, the product in single
/// precision and the differences in double, held to −1.54 (a NaN passes).
pub fn wall_probe(pitch: f32, steps: i32, climb: bool) -> WallProbe {
    let (level, climb) = if steps > 0 && climb {
        (i64::from(steps) - 1, false)
    } else {
        (i64::from(steps), true)
    };
    let lowered = level as f32 * WALL_TILT_STEP;
    let mut probe = ((f64::from(pitch) - WALL_PROBE_LIFT) - f64::from(lowered)) as f32;
    if f64::from(probe) < -1.54 {
        probe = WALL_PITCH_FLOOR;
    }
    WallProbe {
        pitch: probe,
        radius: if climb {
            PROBE_RADIUS_CLIMB
        } else {
            PROBE_RADIUS_DESCEND
        },
        climb,
    }
}

/// The step count after a probe: a hit on a climbing probe climbs (at most
/// four), a miss on a descending probe comes down; anything else keeps the
/// count. Only a descending probe follows a taken step, so the count stays
/// within 0–4.
pub fn wall_steps(steps: i32, climb: bool, obstructed: bool) -> i32 {
    if obstructed {
        if climb && steps < WALL_TILT_MAX {
            return steps + 1;
        }
    } else if !climb {
        return steps - 1;
    }
    steps
}

/// The tilt a step count asks for, `steps · −0.15` in single precision.
pub fn wall_tilt(steps: i32) -> f32 {
    steps as f32 * -WALL_TILT_STEP
}

/// One tick's tilt ease. The change is `target − stored`, a rising change
/// taken at 0.35 (in double), then scaled by `rate` less the last motion
/// (`stored − previous`). A change under 0.005 (compared in double) is
/// added to the last motion as it is; a larger one is held to 0.005 in
/// magnitude in double. The new tilt is `motion · 0.96 + stored`, in
/// double. The caller keeps `stored` as the next `previous`.
pub fn wall_tilt_ease(stored: f32, previous: f32, target: f32, rate: f32) -> f32 {
    let mut change = target - stored;
    let last = stored - previous;
    if change > 0.0 {
        change = (f64::from(change) * TILT_RISE) as f32;
    }
    change = change * rate - last;
    let motion = if f64::from(change.abs()) < TILT_STEP_LIMIT {
        last + change
    } else {
        (TILT_STEP_LIMIT / f64::from(change.abs()) * f64::from(change) + f64::from(last)) as f32
    };
    (f64::from(motion) * TILT_CARRY + f64::from(stored)) as f32
}

/// The pitch the camera's axis is built from: the controller's pitch plus
/// the wall tilt, held to −1.54 (compared in double; a NaN passes).
pub fn tilted_pitch(pitch: f32, tilt: f32) -> f32 {
    let sum = pitch + tilt;
    if f64::from(sum) < -1.54 {
        WALL_PITCH_FLOOR
    } else {
        sum
    }
}

/// The pitch ease of one tick: when the target lies more than 2e-6 away
/// (the gap in single precision, compared in double), the pitch moves a
/// tenth of the way, worked in double; a NaN gap leaves it.
pub fn ease_pitch(pitch: f32, target: f32) -> f32 {
    let gap = target - pitch;
    if f64::from(gap.abs()) > 2e-6 {
        (f64::from(pitch) + f64::from(gap) * 0.1) as f32
    } else {
        pitch
    }
}

/// The backward unit vector of a controller pitch (negative with the eye
/// above the point) and a yaw, as the original's camera-axis helper builds
/// it from the engine's sine and cosine: `(cos p · sin y, −sin p, cos p ·
/// cos y)`.
pub fn camera_axis(pitch: f32, yaw: f32) -> Vec3 {
    let (sin_pitch, cos_pitch) = sin_cos(pitch);
    let (sin_yaw, cos_yaw) = sin_cos(yaw);
    Vec3::new(cos_pitch * sin_yaw, -sin_pitch, cos_pitch * cos_yaw)
}

/// The move one contact puts on a ball: `(point − centre) · (1 − radius /
/// |point − centre|)`, which sets the centre `radius` from the point on
/// the line through both. Single precision, the length summed
/// `(x² + z²) + y²`; a contact at the centre divides by zero, as the
/// original does, and gives NaN.
pub fn hit_offset(centre: Vec3, point: Vec3, radius: f32) -> Vec3 {
    let vector = point - centre;
    vector * (1.0 - radius / length(vector))
}

/// Whether a mesh is near enough for the push-out to ask its triangles: its
/// squared distance (from the scene's test) not beyond the radius squared;
/// a NaN distance passes, as the unordered compare does.
pub fn mesh_in_reach(radius: f32, distance_squared: f32) -> bool {
    let reach = radius * radius;
    reach >= distance_squared || reach.is_nan() || distance_squared.is_nan()
}

/// Lifts a ball out of nearby geometry: each contact the meshes report, in
/// order, moves the centre to `radius` from it ([`hit_offset`]). The
/// original walks the scene's sectors and their flagged meshes in each
/// sector's space ([`mesh_in_reach`] first) and has each mesh call back per
/// contact; the scene walk is the caller's here. The free camera lifts its
/// looked-at point this way with a 30 cm ball after a tick that moved it.
pub fn lift_out_of_geometry(
    centre: Vec3,
    radius: f32,
    contacts: impl IntoIterator<Item = Vec3>,
) -> Vec3 {
    contacts.into_iter().fold(centre, |centre, point| {
        centre + hit_offset(centre, point, radius)
    })
}

/// A frame's backward vector and zoom between the last two ticks: the
/// vectors blended `a + (b − a)·t` and normalised by the reciprocal of
/// their exact length (no guard for zero, as the original's), the zooms
/// blended `(now − before)·t + before`.
pub fn interpolate_eye(before: (Vec3, f32), now: (Vec3, f32), alpha: f32) -> (Vec3, f32) {
    let blend = before.0 + (now.0 - before.0) * alpha;
    (
        blend * (1.0 / length(blend)),
        (now.1 - before.1) * alpha + before.1,
    )
}

/// The word the camera's frame handler raises on the GUI task after it
/// places the view: 0x47 when the camera node needed its matrix rebuilt
/// (its state under 2), else 0x10 unless the camera stayed still through
/// its last tick; `None` when it did.
pub fn frame_notice(rebuilt: bool, still: bool) -> Option<u32> {
    if rebuilt {
        Some(0x47)
    } else if !still {
        Some(0x10)
    } else {
        None
    }
}

/// The field of view (rad) at a zoom: `zoom / divisor · 45 · √(1.6 /
/// aspect)` degrees, the root taken in double precision and the rest in
/// single, then 10° when below 10 and 100° when above 100 (a NaN passes).
/// The stepped camera divides by its extent (2000 in the game).
pub fn fov_from_zoom(zoom: f32, divisor: f32, aspect: f64) -> f32 {
    let scale = (1.6 / aspect).sqrt() as f32;
    let fov = zoom / divisor * 45.0 * scale;
    // `clamp` keeps a NaN, as the original's compares let it pass.
    fov.clamp(10.0, 100.0).to_radians()
}

/// One camera frame's ticks: given the time until the next tick (`ahead`,
/// negative once it has passed), returns how many ticks to run and how far
/// the frame lies into the last one (0–1). At most nine ticks run; a
/// backlog beyond them is dropped by moving the tick clock to now, as the
/// original does.
pub fn frame_step(ahead: &mut f32, dt: f32) -> (u32, f32) {
    *ahead -= dt;
    let mut ran = 0;
    while *ahead < 0.0 && ran < MAX_TICKS {
        *ahead += TICK;
        ran += 1;
    }
    *ahead = ahead.max(0.0);
    (ran, 1.0 - (f64::from(*ahead) / f64::from(TICK)) as f32)
}

/// The constant the camera's preset divides several values by: 59². It is
/// the original's fixed base of the field of view (its 2000 zoom is 45°),
/// and the same divisor scales the preset's view and size.
pub const PRESET_SCALE: f32 = 59.0 * 59.0;

/// A camera preset's raw fields, in the order the original's record keeps
/// them. Their roles were read from the fields the preset writes (guesses
/// where a field only feeds an editor): a zero asks for the default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresetFields {
    /// Flags word. `0x200` installs the wheel's zoom handler, `0x200000`
    /// makes the stepped camera (zoom stops of a fifth of the range, the
    /// zoom's pitch floor and the cut-away probe every frame), `0x100000`
    /// installs the editor's drag, `0x400000` runs the editor's target
    /// scan.
    pub flags: u32,
    /// Field of view scale (default 280).
    pub view: f32,
    /// A share kept per second whose 59th root is kept per 17 ms tick
    /// (default the double 3.94e-13, whose root is 0.6162: the chase
    /// camera's orbit retention; guess for the field's reader, which is
    /// outside this unit).
    pub retention: f32,
    /// Drag sensitivity (default 1).
    pub sensitivity: f32,
    /// Lower pitch bound (default −1.5).
    pub pitch_low: f32,
    /// Upper pitch bound (default 1.5).
    pub pitch_high: f32,
    /// A size whose 59² share is the extent (default 16000).
    pub size: f32,
    /// A rate whose 59th root scales (default 1 when not positive).
    pub rate: f32,
    /// The first distance, 0 meaning derive it (default 0).
    pub distance: f32,
    /// Smallest distance (default 10).
    pub min_distance: f32,
    /// Largest distance (default 10000).
    pub max_distance: f32,
    /// Smallest looked-at height.
    pub min_height: f32,
    /// Largest looked-at height.
    pub max_height: f32,
    /// The record's last field, which the original copies to the
    /// controller as it is (its role was not established).
    pub extra: f32,
}

impl Default for PresetFields {
    fn default() -> Self {
        Self {
            flags: 0,
            view: 0.0,
            retention: 0.0,
            sensitivity: 0.0,
            pitch_low: 0.0,
            pitch_high: 0.0,
            size: 0.0,
            rate: 0.0,
            distance: 0.0,
            min_distance: 0.0,
            max_distance: 0.0,
            min_height: 0.0,
            max_height: 0.0,
            extra: 0.0,
        }
    }
}

/// What a preset puts on the camera controller, with the defaults filled
/// in and the divisions done.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub flags: u32,
    pub view: f32,
    pub retention: f32,
    pub sensitivity: f32,
    pub pitch_low: f32,
    pub pitch_high: f32,
    pub extent: f32,
    pub extent_step: f32,
    pub rate: f32,
    /// The controller's distance field after the minimum clamp: the record's
    /// first distance, or the smallest distance when that is below it.
    pub distance: f32,
    /// Whether the placement derives its distance from the eye instead: the
    /// record's first distance was not positive (the original tests the raw
    /// field, not the clamped one).
    pub derive_distance: bool,
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub extra: f32,
    /// The pitch speed and the zoom ease per tick the preset sets.
    pub pitch_speed: f32,
    pub zoom_speed: f32,
    /// The viewport callbacks the flags install.
    pub handlers: Handlers,
}

/// The viewport callbacks a preset installs, by its flags. The original
/// hands the viewport a wheel handler when the record asks for it; for the
/// editor camera it installs the press and move callbacks, otherwise (when
/// the stepped camera is not the editor's) a free-camera press and, unless
/// the flag says otherwise, a move handler. Exarust has no viewport; the
/// set a record asks for is this value and the host's to install.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Handlers {
    /// The wheel's zoom handler (`0x200`).
    pub wheel: bool,
    /// The editor's press and move drag handlers (`0x100000`).
    pub editor_press: bool,
    pub editor_move: bool,
    /// The free camera's press drag (`0x200000` without the editor bit).
    pub free_press: bool,
    /// The free camera's move handler unless `0x1000000` suppresses it.
    pub free_move: bool,
}

/// The handlers a preset record's flags install.
pub fn preset_handlers(flags: u32) -> Handlers {
    let editor = flags & 0x10_0000 != 0;
    let stepped = flags & 0x20_0000 != 0;
    Handlers {
        wheel: flags & 0x200 != 0,
        editor_press: editor,
        editor_move: editor,
        free_press: !editor && (!stepped || flags & 1 != 0),
        free_move: !editor && flags & 0x100_0000 == 0,
    }
}

/// The field of view scale a preset value gives: `value / 59²`.
pub fn fov_scale(value: f32) -> f32 {
    value / PRESET_SCALE
}

/// The default per-second retention: the original keeps it as a double.
pub const DEFAULT_RETENTION: f64 = 3.94e-13;

/// `exp(ln(value) / 59)`: the preset's 59th-root scaling, as the original
/// does it with the runtime library's logarithm and exponential in double
/// precision.
fn root59(value: f64) -> f32 {
    (value.ln() / 59.0).exp() as f32
}

/// Applies a preset as the original's routine does. The GUI callbacks and
/// the editor's field-of-view viewport are not part of this model.
pub fn apply_preset(fields: PresetFields) -> Preset {
    let size = if fields.size == 0.0 {
        16000.0
    } else {
        fields.size
    };
    let extent = size / PRESET_SCALE;
    let stepped = fields.flags & 0x20_0000 != 0;
    let min_distance = if fields.min_distance > 0.0 {
        fields.min_distance
    } else {
        10.0
    };
    let max_distance = if fields.max_distance > 0.0 {
        fields.max_distance
    } else {
        10000.0
    };
    let rate = if fields.rate > 0.0 {
        root59(f64::from(fields.rate))
    } else {
        1.0
    };
    let distance = if fields.distance < min_distance {
        min_distance
    } else {
        fields.distance
    };
    let pitch_speed = if stepped {
        5.0
    } else {
        max_distance * 15.0 / PRESET_SCALE
    };
    let zoom_speed = if stepped {
        (5.0 / 59.0) as f32
    } else {
        (6.6 / 59.0) as f32
    };
    Preset {
        flags: fields.flags,
        view: fov_scale(if fields.view == 0.0 {
            280.0
        } else {
            fields.view
        }),
        retention: root59(if fields.retention == 0.0 {
            DEFAULT_RETENTION
        } else {
            f64::from(fields.retention)
        }),
        sensitivity: if fields.sensitivity == 0.0 {
            1.0
        } else {
            fields.sensitivity
        },
        pitch_low: if fields.pitch_low == 0.0 {
            -1.5
        } else {
            fields.pitch_low
        },
        pitch_high: if fields.pitch_high == 0.0 {
            1.5
        } else {
            fields.pitch_high
        },
        extent,
        extent_step: (f64::from(extent) * 0.001) as f32,
        rate,
        distance,
        derive_distance: fields.distance <= 0.0,
        min_distance,
        max_distance,
        min_height: fields.min_height,
        max_height: fields.max_height,
        extra: fields.extra,
        pitch_speed,
        zoom_speed,
        handlers: preset_handlers(fields.flags),
    }
}

/// The segment the camera's cut-away driver casts: it starts twice the
/// camera's largest distance out along the backward vector from the
/// looked-at point (past the eye) and runs back towards the point for
/// twice that distance less a metre, so it stops a metre short of the
/// point. The original scales the reversed vector by minus twice the
/// distance. Returns the start, the (unit) direction and the length.
pub fn cut_away_segment(focus: Vec3, back: Vec3, max_distance: f32) -> (Vec3, Vec3, f32) {
    let toward = -back;
    let start = focus + toward * (-max_distance * 2.0);
    (start, toward, max_distance * 2.0 - 1.0)
}

/// The offset one kick puts on the eye, before the chase camera's shake
/// carries it. Three draws make a small random offset (x and z in ±0.5, y
/// in 0–0.15, the last scaled in double precision). The kick's
/// `direction` is the basis; one shorter than √0.1 is replaced by the
/// camera's `right` axis times the sign of a fourth draw less a half, with
/// a fifth draw's `(d − 0.5)·4` added to its height. A basis longer than
/// 2⁻²³ (squared) is normalised by the processor's approximate reciprocal
/// square root, added to the offset, and the sum is scaled to `size`.
///
/// [guess: the original draws from the engine's global generator; Exarust
/// passes its own so a replay keeps its stream]
pub fn kick_offset(
    direction: Vec3,
    right: Vec3,
    size: f32,
    draw: &mut impl FnMut() -> f32,
) -> Vec3 {
    let first = draw() - 0.5;
    let second = (f64::from(draw()) * 0.15) as f32;
    let third = draw() - 0.5;
    let mut offset = Vec3::new(first, second, third);
    let mut basis = direction;
    // The original sums the squares z² + y² + x² in single precision and
    // widens the sum for the 0.1 compare.
    let squared = |v: Vec3| v.z * v.z + v.y * v.y + v.x * v.x;
    if f64::from(squared(basis)) < KICK_DEGENERATE {
        basis = right * sign_or_zero(draw() - 0.5);
        basis.y += (draw() - 0.5) * 4.0;
    }
    if squared(basis) > KICK_EPSILON {
        basis = normalize(basis);
    }
    offset += basis;
    let length_squared = squared(offset);
    if length_squared > KICK_EPSILON {
        offset *= size / length_squared.sqrt();
    }
    offset
}

/// The counts of the original's jittered offset table, group by group: discs
/// of 1, 2, 3, 4, 8, 15, 24 and 66 points, 123 in all.
pub const DISC_GROUP_COUNTS: [usize; 8] = [1, 2, 3, 4, 8, 15, 24, 66];
/// The points in the jittered offset table.
pub const DISC_POINTS: usize = 123;
/// The table's largest radius (its scale is `(draw + 1) · 0.02`).
pub const DISC_SCALE: f64 = 0.02;

/// The original's table of jittered unit offsets, as the unit fills it at
/// start-up: eight groups of a one, two, three, four, eight, fifteen,
/// twenty-four and sixty-six points around a circle, each at
/// `i · 2π / count` with `(sin, cos)` of the engine's fast pair, scaled by
/// `(draw + 1) · 0.02` in double precision and given a z of one. The draw
/// stream is the caller's: the original's is the process-wide generator at
/// the moment of the fill, which a replay cannot reproduce without the
/// earlier draws.
pub fn disc_offsets(draw: &mut impl FnMut() -> f64) -> Vec<Vec3> {
    let mut offsets = Vec::with_capacity(DISC_POINTS);
    for count in DISC_GROUP_COUNTS {
        for i in 0..count {
            let angle = (i as f32 * std::f32::consts::TAU) / count as f32;
            let (sin, cos) = sin_cos(angle);
            let scale = ((draw() + 1.0) * DISC_SCALE) as f32;
            offsets.push(Vec3::new(sin * scale, cos * scale, 1.0));
        }
    }
    offsets
}

/// Moves both of a camera's stored points (the looked-at point and the eye)
/// by `by`, as the unit's translate routine does. Returns the moved pair.
pub fn translate_points(points: (Vec3, Vec3), by: Vec3) -> (Vec3, Vec3) {
    (points.0 + by, points.1 + by)
}

/// A camera controller's attachment to its viewport. The original detaches
/// its controller while the editor's own drag handlers run: the camera node
/// drops its back-pointer to the controller, the editor's press and move
/// callbacks are installed, and the viewport's saved zoom handler is put
/// back on attach. Exarust has no viewport callbacks, so the two observable
/// flags are kept here and the handler swap is the caller's. A new
/// controller is bound (the constructor points the node at it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attachment {
    /// The editor's handlers are installed (`detached`).
    pub editing: bool,
    /// The camera node points back at this controller.
    pub bound: bool,
}

impl Default for Attachment {
    fn default() -> Self {
        Self {
            editing: false,
            bound: true,
        }
    }
}

impl Attachment {
    /// Sets the attachment to `detached` and returns whether it changed.
    /// Detaching installs the editor's handlers and unbinds the node;
    /// attaching binds it again. Setting the state it already has does
    /// nothing, as the original's byte compare does.
    pub fn set_detached(&mut self, detached: bool) -> bool {
        if self.editing == detached {
            return false;
        }
        self.editing = detached;
        self.bound = !detached;
        true
    }
}

/// One node the preset's editor scan reads: the class byte and marker at
/// the node's two flags, its world position and its box's half extents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditorNode {
    /// The node's class byte (`0xA9` is the editor's target).
    pub class: u8,
    /// The node's second class byte (`0xAE` raises the preset's extra flag).
    pub marker: u8,
    pub position: Vec3,
    pub half_extents: Vec3,
}

/// The editor preset's target box: the centre and half extents it scans the
/// scene for, and the raised flag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetBox {
    pub centre: Vec3,
    pub half_extents: Vec3,
    pub raised: bool,
}

/// The editor preset's scan (the preset flag `0x400000`): start from the
/// view's `fallback` centre with an 800 by 0 by 1000 box and no raised flag;
/// every node of class `0xA9` whose class word's low twelve bits are one
/// replaces the box with its position and its x and z half extents (the y
/// extent is zero), and a marker of `0xAE` raises the flag. The last such
/// node in the scene order wins. The class word's low bits are the caller's
/// [`EditorNode::class`] check here.
pub fn scan_target(nodes: &[EditorNode], fallback: Vec3) -> TargetBox {
    let mut target = TargetBox {
        centre: fallback,
        half_extents: Vec3::new(800.0, 0.0, 1000.0),
        raised: false,
    };
    for node in nodes {
        if node.class == 0xA9 && node.marker == 0xAE {
            target.raised = true;
        }
        if node.class == 0xA9 {
            target.centre = node.position;
            target.half_extents = Vec3::new(node.half_extents.x, 0.0, node.half_extents.z);
        }
    }
    target
}

/// The target box the preset asks for, or `None` when its flags do not carry
/// the editor scan's `0x400000`.
pub fn preset_target(
    fields: &PresetFields,
    nodes: &[EditorNode],
    fallback: Vec3,
) -> Option<TargetBox> {
    (fields.flags & 0x40_0000 != 0).then(|| scan_target(nodes, fallback))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn a_target_yaw_stays_within_half_a_turn_of_the_camera() {
        // Within half a turn of the camera: unchanged.
        assert_eq!(align_behind(0.0, 0.0), 0.0);
        assert_eq!(align_behind(0.5, 0.0), 0.5);
        assert_eq!(align_behind(-0.5, 0.0), -0.5);
        // The camera more than π beyond the target: one turn forward.
        let wrapped = align_behind(-3.0, 3.0);
        assert_eq!(wrapped, (f64::from(-3.0f32) + std::f64::consts::TAU) as f32);
        // The camera more than π short of it: one turn back.
        let wrapped = align_behind(3.0, -3.0);
        assert_eq!(wrapped, (f64::from(3.0f32) - std::f64::consts::TAU) as f32);
        // The f32 π is a hair more than the f64 π, so an exact ±π pair
        // wraps by a whole turn (the compare is in f64 over the f32
        // difference).
        let wrapped = align_behind(-PI, 0.0);
        assert!((wrapped - PI).abs() < 1e-6, "{wrapped}");
        let wrapped = align_behind(PI, 0.0);
        assert!((wrapped + PI).abs() < 1e-6, "{wrapped}");
        // A camera that has orbited past a turn pulls the target along.
        let wrapped = align_behind(0.25, 7.0);
        assert_eq!(wrapped, (f64::from(0.25f32) + std::f64::consts::TAU) as f32);
        // A NaN takes neither branch.
        assert!(align_behind(f32::NAN, 0.0).is_nan());
    }

    #[test]
    fn the_planar_yaw_is_the_engine_arc_cosine_of_z() {
        // Straight along +z, +x and −z.
        assert_eq!(planar_yaw(Vec3::Z), 0.0);
        assert_eq!(planar_yaw(-Vec3::Z), PI);
        // x = z = 1: cos 0.70710677 takes the upper form.
        let c = 1.0f32 / 2.0f32.sqrt();
        let upper =
            (c * c * 0.0535 + (std::f32::consts::FRAC_PI_2 - c * 0.2075)) * (1.0 - c).sqrt();
        assert_eq!(planar_yaw(Vec3::new(1.0, 5.0, 1.0)), upper);
        assert_eq!(planar_yaw(Vec3::new(-1.0, 0.0, 1.0)), -upper);
        // Behind (z < 0): π less the lower form, in double precision.
        let lower =
            (c * c * 0.0535 + (-c * 0.2075 + std::f32::consts::FRAC_PI_2)) * (1.0 - c).sqrt();
        let behind = (std::f64::consts::PI - f64::from(lower)) as f32;
        assert_eq!(planar_yaw(Vec3::new(1.0, 0.0, -1.0)), behind);
        // Under 1e-8 of horizontal length squared: zero.
        assert_eq!(planar_yaw(Vec3::new(1e-5, 3.0, 0.0)), 0.0);
        // A NaN falls through to the arc cosine.
        assert!(planar_yaw(Vec3::new(f32::NAN, 0.0, 1.0)).is_nan());
    }

    #[test]
    fn the_orbit_rate_is_4_8_rad_over_the_screen_in_double() {
        assert_eq!(orbit_per_pixel(1920.0), (4.8f64 / 1920.0) as f32);
        assert_eq!(orbit_per_pixel(1920.0), ORBIT_PER_PIXEL);
        assert_eq!(orbit_per_pixel(2560.0), (4.8f64 / 2560.0) as f32);
    }

    #[test]
    fn the_pitch_floor_rises_with_the_zoom() {
        assert_eq!(pitch_floor(0.0), 0.8);
        assert_eq!(pitch_floor(999.0), 0.8);
        assert_eq!(pitch_floor(1000.0), 0.8);
        assert!((pitch_floor(2000.0) - 0.925).abs() < 1e-6);
        assert!((pitch_floor(1300.0) - 0.8375).abs() < 1e-6);
        // The zoom less 1000 is a single, then 0.000125 and 0.8 are doubles.
        assert_eq!(pitch_floor(1000.5), (0.5f64 * 0.000125 + 0.8) as f32);
        assert_eq!(pitch_floor(1000.5), 0.800_062_5);
        let zoom = 9000.3f32;
        assert_eq!(
            pitch_floor(zoom),
            (f64::from(zoom - 1000.0) * 0.000125 + 0.8) as f32
        );
        // An unordered compare takes the steep branch: the floor is NaN.
        assert!(pitch_floor(f32::NAN).is_nan());
    }

    #[test]
    fn the_held_pitch_is_the_asked_one_or_the_steeper_floor() {
        assert_eq!(pitch_held(0.5, 0.8), 0.8);
        assert_eq!(pitch_held(1.2, 0.8), 1.2);
        // No cap here: the drag's clamp holds the asked pitch.
        assert_eq!(pitch_held(2.0, 0.8), 2.0);
        // A NaN floor is unordered: the asked pitch stays.
        assert_eq!(pitch_held(1.0, f32::NAN), 1.0);
        assert!(pitch_held(f32::NAN, 0.8).is_nan());
    }

    #[test]
    fn the_pitch_limits_take_the_branch_the_original_takes() {
        assert_eq!(clamp_pitch(0.5, 0.8, 1.5), 0.8);
        assert_eq!(clamp_pitch(2.0, 0.8, 1.5), 1.5);
        assert_eq!(clamp_pitch(1.0, 0.8, 1.5), 1.0);
        // At the limits the value stays.
        assert_eq!(clamp_pitch(0.8, 0.8, 1.5), 0.8);
        assert_eq!(clamp_pitch(1.5, 0.8, 1.5), 1.5);
        // A NaN is unordered and takes neither side.
        assert!(clamp_pitch(f32::NAN, 0.8, 1.5).is_nan());
    }

    #[test]
    fn an_orbit_drag_moves_2_5_milliradians_a_pixel() {
        // The left drag turns the yaw down, 4.8 rad across 1920 pixels.
        assert!((orbit_yaw(0.0, 1920.0, ORBIT_PER_PIXEL) + 4.8).abs() < 1e-6);
        assert!((orbit_yaw(0.0, 400.0, ORBIT_PER_PIXEL) + 1.0).abs() < 1e-6);
        // A sensitivity halves or doubles it.
        assert!((orbit_yaw(0.0, 400.0, ORBIT_PER_PIXEL * 0.5) + 0.5).abs() < 1e-6);
        // A downward drag raises the pitch, held to 1.5.
        assert_eq!(orbit_pitch(0.8, 400.0, ORBIT_PER_PIXEL, 0.8, 1.5), 1.5);
        assert!((orbit_pitch(0.8, 100.0, ORBIT_PER_PIXEL, 0.8, 1.5) - 1.05).abs() < 1e-6);
        assert_eq!(orbit_pitch(1.4, -400.0, ORBIT_PER_PIXEL, 0.8, 1.5), 0.8);
        // A NaN pitch stays NaN.
        assert!(orbit_pitch(f32::NAN, 1.0, ORBIT_PER_PIXEL, 0.8, 1.5).is_nan());
    }

    #[test]
    fn a_free_zoom_steps_by_a_fifth_of_a_percent_a_unit_or_its_least_step() {
        // One wheel unit: 0.2 % of 1000 is 2, below the 6 of a 1200 range.
        assert_eq!(zoom_steps(1000.0, 1, 800.0, 2000.0), 994.0);
        assert_eq!(zoom_steps(1000.0, -1, 800.0, 2000.0), 1006.0);
        // A zero step stays zero.
        assert_eq!(zoom_steps(1000.0, 0, 800.0, 2000.0), 1000.0);
        // 2000 × 0.002 = 4 < 6, too.
        assert_eq!(zoom_steps(2000.0, 1, 800.0, 2000.0), 1994.0);
        // A notch is 120 units: 24 % of 5000 in a 10–10000 range.
        assert_eq!(zoom_steps(5000.0, 120, 10.0, 10000.0), 3800.0);
        assert_eq!(zoom_steps(5000.0, -120, 10.0, 10000.0), 6200.0);
        // A wide range still takes 0.5 % of it (49.95 as a single).
        assert_eq!(
            zoom_steps(5000.0, 1, 10.0, 10000.0),
            5000.0 - 9990.0f32 * 0.005
        );
        // The result stays in the range.
        assert_eq!(zoom_steps(800.0, 1, 800.0, 2000.0), 800.0);
        assert_eq!(zoom_steps(2000.0, -1, 800.0, 2000.0), 2000.0);
        assert_eq!(zoom_steps(1000.0, 120, 800.0, 2000.0), 800.0);
        // A NaN zoom passes the max-then-min through.
        assert!(zoom_steps(f32::NAN, 1, 800.0, 2000.0).is_nan());
    }

    #[test]
    fn a_placement_clamps_the_height_and_reads_the_back_vector() {
        let limits = HeightLimits {
            min: 10.0,
            max: 100.0,
        };
        let placed = place(
            Vec3::new(1.0, 5.0, 2.0),
            Vec3::new(1.0, 20.0, 7.0),
            None,
            limits,
        );
        assert_eq!(placed.at, Vec3::new(1.0, 10.0, 2.0));
        // The vector runs from the unclamped point: (0, 15, 5).
        let length = (25.0f32 + 225.0).sqrt();
        assert_eq!(placed.distance, length);
        assert_eq!(placed.back, Vec3::new(0.0, 15.0, 5.0) * (1.0 / length));
        // Straight behind: yaw 0, pitch negative (the eye above).
        assert!(placed.pitch < 0.0);
        assert_eq!(placed.pitch, -quick_asin(placed.back.y));
        assert_eq!(placed.yaw, 0.0);
        // The eye on the point: no guard, the vector is NaN.
        let same = place(Vec3::ZERO, Vec3::ZERO, None, limits);
        assert_eq!(same.distance, 0.0);
        assert!(same.back.x.is_nan());
        // A given distance keeps the direction as given.
        let placed = place(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), Some(2000.0), limits);
        assert_eq!(placed.distance, 2000.0);
        assert!((placed.pitch + std::f32::consts::FRAC_PI_2).abs() < 0.01);
        // Heights clamp both ways; a NaN height is unordered and stays.
        let up = place(Vec3::new(0.0, 500.0, 0.0), Vec3::Z, None, limits);
        assert_eq!(up.at.y, 100.0);
        let nan = place(Vec3::new(0.0, f32::NAN, 0.0), Vec3::Z, None, limits);
        assert!(nan.at.y.is_nan());
    }

    #[test]
    fn a_placement_yaw_is_the_engine_yaw_of_the_back_vector() {
        // Behind +X: the engine's planar yaw gives its arc cosine of 0.
        let placed = place(
            Vec3::ZERO,
            Vec3::X,
            None,
            HeightLimits {
                min: -1e9,
                max: 1e9,
            },
        );
        assert_eq!(placed.yaw, std::f32::consts::FRAC_PI_2);
        // Behind −Z: half a turn.
        let placed = place(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -1.0),
            None,
            HeightLimits {
                min: -1e9,
                max: 1e9,
            },
        );
        assert!((placed.yaw - PI).abs() < 1e-6);
    }

    #[test]
    fn a_fixed_distance_stays_between_its_bounds() {
        assert_eq!(clamp_distance(5.0, 10.0, 10000.0), 10.0);
        assert_eq!(clamp_distance(20000.0, 10.0, 10000.0), 10000.0);
        assert_eq!(clamp_distance(2000.0, 10.0, 10000.0), 2000.0);
        assert_eq!(clamp_fixed_pitch(0.5, 0.8, 1.5), 0.8);
        assert_eq!(clamp_fixed_pitch(2.0, 0.8, 1.5), 1.5);
        assert_eq!(clamp_fixed_pitch(1.0, 0.8, 1.5), 1.0);
    }

    #[test]
    fn a_frame_runs_at_most_nine_ticks_and_blends_the_rest() {
        let mut ahead = 0.0;
        // 17 ms is exactly one tick: the frame starts the next.
        let (ran, t) = frame_step(&mut ahead, 0.017);
        assert_eq!(ran, 1);
        assert!((t - 1.0).abs() < 1e-6);
        // Half a tick into the next.
        let mut ahead = -0.0085;
        let (ran, t) = frame_step(&mut ahead, 0.0);
        assert_eq!(ran, 1);
        assert!((t - 0.5).abs() < 1e-6);
        // A 1 s stall runs nine ticks and snaps the clock to now.
        let mut ahead = 0.0;
        let (ran, t) = frame_step(&mut ahead, 1.0);
        assert_eq!((ran, t), (9, 1.0));
        assert_eq!(ahead, 0.0);
    }

    #[test]
    fn a_preset_fills_the_defaults_and_scales_by_59() {
        let preset = apply_preset(PresetFields::default());
        assert!(
            (preset.view - 0.080_436_656).abs() < 1e-6,
            "{}",
            preset.view
        );
        // The default retention is the double 3.94e-13: its 59th root is
        // the chase camera's 0.616 orbit retention.
        assert_eq!(preset.retention, (3.94e-13f64.ln() / 59.0).exp() as f32);
        assert!(
            (preset.retention - 0.616_246_1).abs() < 1e-6,
            "{}",
            preset.retention
        );
        assert!((preset.extent - 4.596_380_4).abs() < 1e-6);
        assert!((preset.extent_step - 0.004_596_380_4).abs() < 1e-9);
        assert!((preset.rate - 1.0).abs() < 1e-6);
        assert_eq!(
            (preset.sensitivity, preset.pitch_low, preset.pitch_high),
            (1.0, -1.5, 1.5)
        );
        assert_eq!((preset.min_distance, preset.max_distance), (10.0, 10000.0));
        // A zero first distance is held up to the minimum for the field,
        // but the placement still derives its own distance.
        assert_eq!(preset.distance, 10.0);
        assert!(preset.derive_distance);
        assert!((preset.pitch_speed - 43.091_064).abs() < 1e-4);
        assert!((preset.zoom_speed - 0.111_864_41).abs() < 1e-6);
        // The stepped camera swaps in its own speeds.
        let fixed = apply_preset(PresetFields {
            flags: 0x20_0000,
            ..PresetFields::default()
        });
        assert_eq!(fixed.pitch_speed, 5.0);
        assert!((fixed.zoom_speed - 5.0 / 59.0).abs() < 1e-6);
        // The stepped record installs no wheel handler (its 0x200 is not
        // set) but takes the free press only with the plain-camera bit.
        assert!(!fixed.handlers.wheel);
        assert!(!fixed.handlers.free_press);
    }

    #[test]
    fn a_preset_keeps_nonzero_fields_and_takes_the_roots() {
        let preset = apply_preset(PresetFields {
            flags: 0x200,
            view: 100.0,
            retention: 8.0,
            sensitivity: 2.0,
            pitch_low: -0.5,
            pitch_high: 0.75,
            size: 3481.0,
            rate: 59.0,
            distance: 2000.0,
            min_distance: 1.0,
            max_distance: 5000.0,
            min_height: 10.0,
            max_height: 20.0,
            extra: 3.0,
        });
        assert!((preset.view - 100.0 / 3481.0).abs() < 1e-6);
        assert_eq!(preset.retention, (8f64.ln() / 59.0).exp() as f32);
        assert_eq!(preset.sensitivity, 2.0);
        assert_eq!((preset.pitch_low, preset.pitch_high), (-0.5, 0.75));
        assert_eq!(preset.extent, 1.0);
        assert!((preset.rate - 59f32.powf(1.0 / 59.0)).abs() < 1e-5);
        assert_eq!(preset.distance, 2000.0);
        assert!(!preset.derive_distance);
        assert_eq!((preset.min_distance, preset.max_distance), (1.0, 5000.0));
        assert_eq!(
            (preset.min_height, preset.max_height, preset.extra),
            (10.0, 20.0, 3.0)
        );
        // A non-positive rate keeps the default 1.
        let preset = apply_preset(PresetFields {
            rate: -1.0,
            ..PresetFields::default()
        });
        assert_eq!(preset.rate, 1.0);
        assert_eq!(fov_scale(3481.0), 1.0);
    }

    #[test]
    fn the_cut_away_segment_starts_beyond_the_eye_and_stops_short() {
        // The eye is 20 m out along +Z; the segment starts 40 m out and
        // runs back towards the point for 39 m.
        let (start, direction, length) =
            cut_away_segment(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, 0.0, 1.0), 20.0);
        assert_eq!(start, Vec3::new(0.0, 10.0, 40.0));
        assert_eq!(direction, -Vec3::Z);
        assert_eq!(length, 39.0);
        assert_eq!(start + direction * length, Vec3::new(0.0, 10.0, 1.0));
        // A small distance can make a negative length; the detector's
        // segment then runs the other way, as the original's does.
        let (_, _, length) = cut_away_segment(Vec3::ZERO, Vec3::Z, 0.4);
        assert!(length < 0.0);
    }

    #[test]
    fn a_wall_probe_alternates_between_the_step_and_the_one_below() {
        assert_eq!(probe_length(2000.0), 3850.0);
        // No step taken: the present step, 0.05 higher, climbing, 30 cm.
        let probe = wall_probe(-0.3, 0, true);
        assert!(probe.climb);
        assert_eq!(probe.radius, 30.0);
        assert_eq!(probe.pitch, (f64::from(-0.3f32) - 0.05) as f32);
        // Two steps with the flag set: the step below, descending, 20 cm.
        let probe = wall_probe(-0.3, 2, true);
        assert!(!probe.climb);
        assert_eq!(probe.radius, 20.0);
        let lowered = f64::from(0.15f32);
        assert_eq!(probe.pitch, ((f64::from(-0.3f32) - 0.05) - lowered) as f32);
        // Two steps with the flag clear: the present step again.
        let probe = wall_probe(-0.3, 2, false);
        assert!(probe.climb);
        let lowered = f64::from(2.0f32 * 0.15);
        assert_eq!(probe.pitch, ((f64::from(-0.3f32) - 0.05) - lowered) as f32);
        // Held to −1.54; a NaN passes.
        assert_eq!(wall_probe(-1.5, 0, false).pitch, -1.54);
        assert!(wall_probe(f32::NAN, 0, false).pitch.is_nan());
        // A climbing hit climbs to four at most; a descending miss comes
        // down; the other two keep the count.
        assert_eq!(wall_steps(0, true, true), 1);
        assert_eq!(wall_steps(4, true, true), 4);
        assert_eq!(wall_steps(2, false, false), 1);
        assert_eq!(wall_steps(2, true, false), 2);
        assert_eq!(wall_steps(2, false, true), 2);
        assert_eq!(wall_tilt(3), 3.0 * -0.15f32);
        // The axis takes the tilt, held to −1.54.
        assert_eq!(tilted_pitch(-0.5, -0.3), -0.8);
        assert_eq!(tilted_pitch(-1.4, -0.6), -1.54);
        assert_eq!(tilted_pitch(-0.5, 0.0), -0.5);
    }

    #[test]
    fn the_wall_tilt_eases_by_at_most_5_mrad_a_tick() {
        // From rest toward one step: the change −0.15 is held to −0.005,
        // and 0.96 of it is taken.
        let tilt = wall_tilt_ease(0.0, 0.0, -0.15, 1.0);
        let held = (0.005 / f64::from(0.15f32) * f64::from(-0.15f32)) as f32;
        assert_eq!(tilt, (f64::from(held) * 0.96) as f32);
        // A small change passes as it is.
        let tilt = wall_tilt_ease(0.0, 0.0, -0.003, 1.0);
        assert_eq!(tilt, (f64::from(-0.003f32) * 0.96) as f32);
        // A rising change is taken at 0.35 first: 0.01 becomes 0.0035.
        let tilt = wall_tilt_ease(-0.01, -0.01, 0.0, 1.0);
        let rise = (f64::from(0.01f32) * 0.35) as f32;
        assert_eq!(tilt, (f64::from(rise) * 0.96 + f64::from(-0.01f32)) as f32);
        // At rest on its target it stays.
        assert_eq!(wall_tilt_ease(-0.15, -0.15, -0.15, 1.0), -0.15);
        // On a still target the last motion is taken back: a motion of
        // −0.004 and a change of +0.004 cancel.
        let tilt = wall_tilt_ease(-0.1, -0.096, -0.1, 1.0);
        let last = -0.1f32 - -0.096f32;
        // The still target's change is 0, at a rate of 1, less the motion.
        let change = -last;
        assert_eq!(
            tilt,
            (f64::from(last + change) * 0.96 + f64::from(-0.1f32)) as f32
        );
    }

    #[test]
    fn the_pitch_eases_a_tenth_in_double_over_a_small_gap() {
        let eased = (f64::from(0.8f32) + f64::from(0.9f32 - 0.8f32) * 0.1) as f32;
        assert_eq!(ease_pitch(0.8, 0.9), eased);
        assert_eq!(ease_pitch(0.8, 0.8 + 1e-6), 0.8);
        assert!(ease_pitch(f32::NAN, 0.8).is_nan());
        assert_eq!(ease_pitch(0.8, f32::NAN), 0.8);
    }

    #[test]
    fn a_contact_sets_the_ball_its_radius_away() {
        let offset = hit_offset(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), 2.0);
        assert_eq!(offset, Vec3::new(0.0, 0.0, 8.0));
        assert_eq!(
            hit_offset(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), 10.0),
            Vec3::ZERO
        );
        assert!(hit_offset(Vec3::Z, Vec3::Z, 1.0).x.is_nan());
        // A surface point at x = 1 inside a 4 cm ball at x = 2 sets the
        // centre at x = 5; a second contact 2 above it then sets it 4
        // below that one.
        let lifted = lift_out_of_geometry(Vec3::new(2.0, 0.0, 0.0), 4.0, [Vec3::X]);
        assert_eq!(lifted, Vec3::new(5.0, 0.0, 0.0));
        let lifted = lift_out_of_geometry(
            Vec3::new(2.0, 0.0, 0.0),
            4.0,
            [Vec3::X, Vec3::new(5.0, 2.0, 0.0)],
        );
        assert_eq!(lifted, Vec3::new(5.0, -2.0, 0.0));
        assert_eq!(lift_out_of_geometry(Vec3::X, 4.0, []), Vec3::X);
        assert!(mesh_in_reach(2.0, 4.0));
        assert!(!mesh_in_reach(2.0, 4.5));
        assert!(mesh_in_reach(2.0, f32::NAN));
        assert_eq!(frame_notice(true, true), Some(0x47));
        assert_eq!(frame_notice(false, false), Some(0x10));
        assert_eq!(frame_notice(false, true), None);
    }

    #[test]
    fn the_camera_axis_places_the_back_vector() {
        let axis = camera_axis(0.0, 0.0);
        assert!((axis - Vec3::Z).length() < 1e-6);
        let axis = camera_axis(0.0, std::f32::consts::FRAC_PI_2);
        assert!((axis - Vec3::X).length() < 1e-6);
        // A negative pitch puts the eye above the point.
        let axis = camera_axis(-std::f32::consts::FRAC_PI_2, 0.0);
        assert!((axis - Vec3::Y).length() < 1e-6);
        let (back, zoom) = interpolate_eye((Vec3::Z, 800.0), (Vec3::X, 2000.0), 0.5);
        assert!((back - Vec3::new(0.70710677, 0.0, 0.70710677)).length() < 1e-6);
        assert_eq!(zoom, 1400.0);
        // Opposite vectors meet at zero: no guard, as the original's.
        assert!(
            interpolate_eye((Vec3::Z, 0.0), (-Vec3::Z, 0.0), 0.5)
                .0
                .x
                .is_nan()
        );
        assert!((fov_from_zoom(2000.0, 2000.0, 1.6).to_degrees() - 45.0).abs() < 1e-3);
        assert_eq!(fov_from_zoom(1.0, 2000.0, 1.6), 10f32.to_radians());
        assert_eq!(fov_from_zoom(1e9, 1.0, 1.6), 100f32.to_radians());
        // A zero aspect makes an infinite view, held to 100°.
        assert_eq!(fov_from_zoom(2000.0, 2000.0, 0.0), 100f32.to_radians());
    }

    #[test]
    fn a_kick_is_its_direction_plus_a_random_offset_scaled_to_its_size() {
        // Draws 0.75, 0.5, 0.5: the offset (0.25, 0.075, 0). A unit
        // direction along −Z (the approximate reciprocal square root of 1
        // is near 1) is added and the sum scaled to 2 m.
        let mut draws = [0.75, 0.5, 0.5].into_iter();
        let offset = kick_offset(-Vec3::Z, Vec3::X, 2.0, &mut || draws.next().unwrap());
        let basis = normalize(-Vec3::Z);
        let sum = Vec3::new(0.25, (0.5f64 * 0.15) as f32, 0.0) + basis;
        assert_eq!(offset, sum * (2.0 / sum.length_squared().sqrt()));
        assert!(draws.next().is_none(), "three draws only");
        // A long direction is normalised first.
        let mut draws = [0.5, 0.0, 0.5].into_iter();
        let offset = kick_offset(Vec3::Y * 10.0, Vec3::X, 1.0, &mut || draws.next().unwrap());
        assert!((offset - Vec3::Y).length() < 1e-3, "{offset:?}");
        // A degenerate direction takes the right axis by a fourth draw's
        // sign and a fifth draw's height: 0.25 → −X, 0.75 → +1 up.
        let mut draws = [0.5, 0.0, 0.5, 0.25, 0.75].into_iter();
        let offset = kick_offset(Vec3::ZERO, Vec3::X, 3.0, &mut || draws.next().unwrap());
        let basis = normalize(Vec3::new(-1.0, 1.0, 0.0));
        assert_eq!(offset, basis * (3.0 / basis.length_squared().sqrt()));
        assert!(draws.next().is_none());
    }

    #[test]
    fn the_jittered_table_rings_its_points_at_two_to_four_centimetres() {
        let offsets = disc_offsets(&mut || 0.0);
        assert_eq!(offsets.len(), DISC_POINTS);
        // The first ring's only point is 2 cm up +Z with the table's unit
        // height, and every point has that height.
        assert!((offsets[0] - Vec3::new(0.0, 0.02, 1.0)).length() < 1e-6);
        assert!(offsets.iter().all(|p| p.z == 1.0));
        // The second ring's points are opposite: 2 cm up and 2 cm down.
        assert!((offsets[1] - Vec3::new(0.0, 0.02, 1.0)).length() < 1e-6);
        assert!((offsets[2] - Vec3::new(0.0, -0.02, 1.0)).length() < 1e-6);
        // The draws scale the ring: a draw of 1 makes the radius 4 cm.
        let offsets = disc_offsets(&mut || 1.0);
        assert!((offsets[0] - Vec3::new(0.0, 0.04, 1.0)).length() < 1e-6);
        // A draw of −1 is the zero radius (the original never draws it).
        assert_eq!(disc_offsets(&mut || -1.0)[0], Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn both_stored_points_move_together() {
        let points = (Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0));
        let moved = translate_points(points, Vec3::new(0.5, -1.0, 2.0));
        assert_eq!(moved.0, Vec3::new(1.5, 1.0, 5.0));
        assert_eq!(moved.1, Vec3::new(4.5, 4.0, 8.0));
    }

    #[test]
    fn a_preset_installs_the_handlers_its_flags_ask_for() {
        // The plain free camera: no wheel, the free press (no stepped flag)
        // and the free move.
        let handlers = preset_handlers(0);
        assert_eq!(
            handlers,
            Handlers {
                wheel: false,
                editor_press: false,
                editor_move: false,
                free_press: true,
                free_move: true,
            }
        );
        // The wheel and the stepped camera: the press needs the plain-camera
        // bit, the move is suppressed by 0x1000000.
        let handlers = preset_handlers(0x20_0000 | 0x200 | 0x100_0000);
        assert!(handlers.wheel);
        assert!(!handlers.free_press);
        assert!(!handlers.free_move);
        let handlers = preset_handlers(0x20_0000 | 1);
        assert!(handlers.free_press);
        // The editor takes the press and move; the free handlers stay off.
        let handlers = preset_handlers(0x10_0000 | 0x20_0000);
        assert!(handlers.editor_press && handlers.editor_move);
        assert!(!handlers.free_press && !handlers.free_move);
    }

    #[test]
    fn a_detach_swaps_the_handlers_once() {
        let mut attachment = Attachment::default();
        assert!(attachment.set_detached(true));
        assert!(attachment.editing && !attachment.bound);
        // Setting the state it already has changes nothing.
        assert!(!attachment.set_detached(true));
        assert!(attachment.editing && !attachment.bound);
        assert!(attachment.set_detached(false));
        assert!(!attachment.editing && attachment.bound);
        assert!(!attachment.set_detached(false));
    }

    #[test]
    fn the_editor_preset_scans_for_its_target_box() {
        let view = Vec3::new(1.0, 2.0, 3.0);
        let empty = scan_target(&[], view);
        assert_eq!(empty.centre, view);
        assert_eq!(empty.half_extents, Vec3::new(800.0, 0.0, 1000.0));
        assert!(!empty.raised);
        let nodes = [
            EditorNode {
                class: 0xA9,
                marker: 0,
                position: Vec3::new(10.0, 20.0, 30.0),
                half_extents: Vec3::new(5.0, 6.0, 7.0),
            },
            EditorNode {
                class: 0xA8,
                marker: 0xAE,
                position: Vec3::ZERO,
                half_extents: Vec3::ZERO,
            },
            EditorNode {
                class: 0xA9,
                marker: 0xAE,
                position: Vec3::new(40.0, 50.0, 60.0),
                half_extents: Vec3::new(8.0, 9.0, 10.0),
            },
        ];
        let target = scan_target(&nodes, view);
        // The last 0xA9 wins, with a zero height; the 0xAE marks it raised.
        assert_eq!(target.centre, Vec3::new(40.0, 50.0, 60.0));
        assert_eq!(target.half_extents, Vec3::new(8.0, 0.0, 10.0));
        assert!(target.raised);
        // The marker alone does not raise the flag; it rides on the class.
        let target = scan_target(&nodes[..2], view);
        assert_eq!(target.centre, Vec3::new(10.0, 20.0, 30.0));
        assert!(!target.raised);
        // The preset only scans under its editor flag.
        assert!(preset_target(&PresetFields::default(), &nodes, view).is_none());
        let fields = PresetFields {
            flags: 0x40_0000,
            ..PresetFields::default()
        };
        let target = preset_target(&fields, &nodes, view).unwrap();
        assert_eq!(target.centre, Vec3::new(40.0, 50.0, 60.0));
        assert!(target.raised);
        assert_eq!(preset_target(&fields, &[], view).unwrap().centre, view);
    }
}
