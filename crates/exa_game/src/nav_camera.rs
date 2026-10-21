//! The original's camera navigation routines: where the view sits, the yaw
//! that turns it behind a body, the orbit drag and its pitch limits, the
//! zoom steps, the frame clock and a kick. The chase camera in
//! [`crate::player`] uses these; the rules they reproduce are in
//! `docs/PLAYER.md` (Camera).
//!
//! The original keeps its camera state in one controller and its arithmetic
//! in a navigation unit. This module is that unit: plain functions, no
//! state, so the chase camera's fields keep the controller's job.

use bevy::math::Vec3;
use exa_ai::search::quick_asin;

use crate::thaum::engine::normalize;

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
/// The deepest the raised orbit may lie (rad).
pub const WALL_PITCH_FLOOR: f32 = -1.54;
/// The obstruction probe's radius, per the raised flag: 30 or 20.
pub const PROBE_RADIUS_RAISED: f32 = 30.0;
pub const PROBE_RADIUS_PLAIN: f32 = 20.0;
/// The pitch offset the probe starts from.
pub const WALL_PITCH_START: f32 = 0.05;

/// The obstruction probe's length from the camera's extent: twice the
/// extent less 150, as the original's `extent · 2 − 150`.
pub fn probe_length(extent: f32) -> f32 {
    extent * 2.0 - 150.0
}

/// The probe's radius: 30 with the raised flag, else 20.
pub fn probe_radius(raised: bool) -> f32 {
    if raised {
        PROBE_RADIUS_RAISED
    } else {
        PROBE_RADIUS_PLAIN
    }
}

/// The pitch the probe looks down from: the camera pitch less 0.05, less
/// one step per tilt already taken (or one fewer while the raised flag is
/// set), held to −1.54. The original tests `steps < 1` first, so a
/// non-positive count uses the un-raised form.
pub fn wall_probe_pitch(pitch: f32, steps: i32, raised: bool) -> f32 {
    let offset = if steps < 1 || !raised {
        steps as f32
    } else {
        (steps - 1) as f32
    };
    let value = (pitch - WALL_PITCH_START) - offset * WALL_TILT_STEP;
    if value < WALL_PITCH_FLOOR {
        WALL_PITCH_FLOOR
    } else {
        value
    }
}

/// The tilt a step count stores, `−steps · 0.15`.
pub fn wall_tilt(steps: i32) -> f32 {
    -(steps as f32) * WALL_TILT_STEP
}

/// The next tilt step count: a raised camera that sees no obstruction
/// lowers its count (never below 0); a raised camera that sees one climbs
/// (at most four). A camera that is not raised never moves.
pub fn wall_steps(steps: i32, raised: bool, obstructed: bool) -> i32 {
    if obstructed {
        if raised && steps < WALL_TILT_MAX {
            return steps + 1;
        }
    } else if raised {
        return steps - 1;
    }
    steps
}

/// One tilt ease: `target` less the stored tilt, half again when rising,
/// worked through the previous frame's change, scaled by the rate, the
/// change capped at 0.005 in magnitude, and the tilt moved by
/// `(previous change + change) · 0.96`. Returns the new tilt and the
/// change the next frame measures from.
pub fn wall_tilt_ease(stored: f32, previous: f32, target: f32, rate: f32) -> (f32, f32) {
    let mut change = target - stored;
    let last = stored - previous;
    if change > 0.0 {
        change *= 0.35;
    }
    change = change * rate - last;
    if change.abs() >= 0.005 {
        change = 0.005 / change.abs() * change;
    }
    let tilt = (last + change) * 0.96 + stored;
    (tilt, change)
}

/// The backward unit vector of a camera pitch and yaw, as the original's
/// camera-axis helper: the cosine of the pitch across by the sine and
/// cosine of the yaw, and minus the pitch's sine up. [guess: the helper's
/// two arguments are the pitch and yaw; their order is the one its callers
/// pass]
pub fn camera_axis(pitch: f32, yaw: f32) -> Vec3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    Vec3::new(cos_pitch * sin_yaw, -sin_pitch, cos_pitch * cos_yaw)
}

/// The obstruction probe's segment: it starts a radius back from the focus
/// along the probe direction and runs the probe's length. The original
/// offsets the look-at point by the radius before casting.
pub fn probe_segment(focus: Vec3, direction: Vec3, radius: f32, length: f32) -> (Vec3, Vec3) {
    (
        focus + direction * radius,
        focus + direction * (radius + length),
    )
}

/// Whether a swept sphere meets the scene: the original casts from the
/// focus along the probe direction with the probe's radius and length and
/// asks each mesh's own test. The mesh walk is a closure here.
pub fn sweep_obstructed(
    from: Vec3,
    to: Vec3,
    radius: f32,
    mut hit: impl FnMut(Vec3, Vec3, f32) -> bool,
) -> bool {
    hit(from, to, radius)
}

/// Lifts a point out of nearby geometry: for every surface point a probe
/// returns within `radius`, the point is pushed away from it by `radius`.
/// The original walks the scene's sectors and asks each mesh's probe; this
/// is the same push with the mesh walk supplied as a closure. [guess: the
/// push direction is away from the nearest surface point]
pub fn lift_out_of_geometry(
    point: Vec3,
    radius: f32,
    mut nearest: impl FnMut(Vec3, f32) -> Option<Vec3>,
) -> Vec3 {
    let Some(surface) = nearest(point, radius) else {
        return point;
    };
    let away = point - surface;
    if away.length_squared() > 1e-12 {
        point + away.normalize() * radius
    } else {
        point
    }
}

/// A frame's eye between the last two ticks: the backward vectors blended
/// and normalised and the zooms blended, as the original's interpolated
/// placement does.
pub fn interpolate_eye(before: (Vec3, f32), now: (Vec3, f32), alpha: f32) -> (Vec3, f32) {
    (
        before.0.lerp(now.0, alpha).normalize_or(Vec3::Z),
        before.1 + (now.1 - before.1) * alpha,
    )
}

/// The intersection callback's adjusted offset: `(to − from) · (1 −
/// radius / |to − from|)`, the original's subtract, length and scale in
/// single precision. A zero-length vector divides by zero, as the original
/// does, and the result is NaN.
pub fn hit_offset(from: Vec3, to: Vec3, radius: f32) -> Vec3 {
    let vector = to - from;
    let length = vector.length();
    vector * (1.0 - radius / length)
}

/// The words a released camera node raises on the current layer: 0x47 when
/// the node was updated (its state was under 2), else 0x10 unless the node
/// carries its leaving flag. `None` when the node was not updated and the
/// flag is set.
pub fn release_notice(updated: bool, leaving: bool) -> Option<u32> {
    if updated {
        Some(0x47)
    } else if !leaving {
        Some(0x10)
    } else {
        None
    }
}

/// The field of view (rad) at a zoom: `zoom / divisor · 45° · √(1.6 /
/// aspect)`, held between 10° and 100°. The original divides the zoom by
/// the preset's view scale and clamps the same way.
pub fn fov_from_zoom(zoom: f32, divisor: f32, aspect: f32) -> f32 {
    (zoom / divisor * 45.0 * (1.6 / aspect.max(0.1)).sqrt())
        .clamp(10.0, 100.0)
        .to_radians()
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
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub extra: f32,
    /// The pitch speed and the zoom ease per tick the preset sets.
    pub pitch_speed: f32,
    pub zoom_speed: f32,
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
        distance: fields.distance,
        min_distance,
        max_distance,
        min_height: fields.min_height,
        max_height: fields.max_height,
        extra: fields.extra,
        pitch_speed,
        zoom_speed,
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
    if f64::from(basis.length_squared()) < KICK_DEGENERATE {
        basis = right * sign_or_zero(draw() - 0.5);
        basis.y += (draw() - 0.5) * 4.0;
    }
    if basis.length_squared() > KICK_EPSILON {
        basis = normalize(basis);
    }
    offset += basis;
    let length_squared = offset.length_squared();
    if length_squared > KICK_EPSILON {
        offset *= size / length_squared.sqrt();
    }
    offset
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
        assert!((preset.pitch_speed - 43.091_064).abs() < 1e-4);
        assert!((preset.zoom_speed - 0.111_864_41).abs() < 1e-6);
        // The stepped camera swaps in its own speeds.
        let fixed = apply_preset(PresetFields {
            flags: 0x20_0000,
            ..PresetFields::default()
        });
        assert_eq!(fixed.pitch_speed, 5.0);
        assert!((fixed.zoom_speed - 5.0 / 59.0).abs() < 1e-6);
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
    fn the_wall_probe_rises_and_eases_into_its_tilt() {
        assert_eq!(probe_length(100.0), 50.0);
        assert_eq!(probe_radius(true), 30.0);
        assert_eq!(probe_radius(false), 20.0);
        // 0.3 − 0.05 = 0.25; the raised flag with one step takes none off.
        assert!((wall_probe_pitch(0.3, 1, true) - 0.25).abs() < 1e-6);
        // Two steps take one off: 0.10.
        assert!((wall_probe_pitch(0.3, 2, true) - 0.10).abs() < 1e-6);
        // Not raised: every step counts, and a negative count adds pitch.
        assert!((wall_probe_pitch(0.3, 2, false) - (0.25 - 0.3)).abs() < 1e-6);
        assert!((wall_probe_pitch(0.3, -1, false) - 0.4).abs() < 1e-6);
        // Held to −1.54.
        assert_eq!(wall_probe_pitch(-2.0, 4, false), -1.54);
        assert!((wall_tilt(3) + 0.45).abs() < 1e-6);
        // A raised camera climbs on a hit, lowers without, and stops at 4.
        assert_eq!(wall_steps(0, true, true), 1);
        assert_eq!(wall_steps(4, true, true), 4);
        assert_eq!(wall_steps(2, true, false), 1);
        assert_eq!(wall_steps(0, true, false), -1);
        // A plain camera never moves.
        assert_eq!(wall_steps(2, false, true), 2);
        assert_eq!(wall_steps(2, false, false), 2);
        // The ease: from 0 toward −0.15, a rate of 1, previous 0.
        let (tilt, change) = wall_tilt_ease(0.0, 0.0, -0.15, 1.0);
        assert_eq!(change, -0.005);
        assert!((tilt + 0.0048).abs() < 1e-6);
        // A rising change is taken at 0.35 first.
        let (_, change) = wall_tilt_ease(0.0, 0.0, 1.0, 1.0);
        assert_eq!(change, 0.005);
        // A still target still moves by the previous change: the change
        // is the previous tilt less the previous previous tilt.
        let (tilt, change) = wall_tilt_ease(-1.0, 0.0, -1.0, 1.0);
        assert_eq!(change, 0.005);
        assert!((tilt - (-0.9552 - 1.0)).abs() < 1e-5, "{tilt}");
    }

    #[test]
    fn the_intersection_offset_pulls_back_by_the_radius() {
        let offset = hit_offset(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), 2.0);
        assert!((offset - Vec3::new(0.0, 0.0, 8.0)).length() < 1e-6);
        assert_eq!(
            hit_offset(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), 10.0),
            Vec3::ZERO
        );
        assert!(hit_offset(Vec3::Z, Vec3::Z, 1.0).x.is_nan());
        assert_eq!(release_notice(true, true), Some(0x47));
        assert_eq!(release_notice(false, false), Some(0x10));
        assert_eq!(release_notice(false, true), None);
        assert!(sweep_obstructed(
            Vec3::ZERO,
            Vec3::Z,
            20.0,
            |from, to, radius| { (from, to, radius) == (Vec3::ZERO, Vec3::Z, 20.0) }
        ));
        assert!(!sweep_obstructed(Vec3::ZERO, Vec3::Z, 20.0, |_, _, _| {
            false
        }));
    }

    #[test]
    fn the_camera_axis_places_the_back_vector() {
        let axis = camera_axis(0.0, 0.0);
        assert!((axis - Vec3::Z).length() < 1e-6);
        let axis = camera_axis(0.0, std::f32::consts::FRAC_PI_2);
        assert!((axis - Vec3::X).length() < 1e-6);
        let axis = camera_axis(std::f32::consts::FRAC_PI_2, 0.0);
        assert!((axis - -Vec3::Y).length() < 1e-6);
        let (start, end) = probe_segment(Vec3::Z, Vec3::Z, 20.0, 30.0);
        assert_eq!(start, Vec3::new(0.0, 0.0, 21.0));
        assert_eq!(end, Vec3::new(0.0, 0.0, 51.0));
        // A surface at x = 1 pushes a point at x = 2 out by the radius.
        let lifted = lift_out_of_geometry(Vec3::new(2.0, 0.0, 0.0), 4.0, |_, _| {
            Some(Vec3::new(1.0, 0.0, 0.0))
        });
        assert_eq!(lifted, Vec3::new(6.0, 0.0, 0.0));
        assert_eq!(lift_out_of_geometry(Vec3::X, 4.0, |_, _| None), Vec3::X);
        let (back, zoom) = interpolate_eye((Vec3::Z, 800.0), (Vec3::X, 2000.0), 0.5);
        assert!((back - Vec3::new(0.70710677, 0.0, 0.70710677)).length() < 1e-6);
        assert_eq!(zoom, 1400.0);
        assert!((fov_from_zoom(2000.0, 2000.0, 1.6).to_degrees() - 45.0).abs() < 1e-3);
        assert_eq!(fov_from_zoom(1.0, 2000.0, 1.6).to_degrees(), 10.0);
        assert_eq!(fov_from_zoom(1e9, 1.0, 1.6).to_degrees(), 100.0);
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
}
