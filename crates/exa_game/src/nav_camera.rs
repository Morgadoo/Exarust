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
use exa_combat::aim::head_atan2_xz;

/// Orbit and camera-look sensitivity: 4.8 rad across a screen 1920 pixels
/// wide, as the original sets it from the window width (rad per pixel).
pub const ORBIT_PER_PIXEL: f32 = 0.0025;
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
/// A free zoom moves by this share of the current value per wheel step.
pub const ZOOM_RATE: f32 = 0.002;
/// A free zoom's smallest step: this share of the whole range.
pub const ZOOM_STEP_SHARE: f32 = 0.005;
/// A kick larger than this (squared) is normalised to its size.
const KICK_EPSILON: f32 = 1.192_089_982_282_595_9e-7;

/// Keeps a camera yaw within half a turn of a body's yaw, as the original's
/// align-behind routine does: `camera + 2π` when it lies more than π behind
/// the body, `camera − 2π` when it lies more than π ahead. The sums are
/// done in double precision, as the original converts to `f64` for the
/// comparisons and the turn. A NaN yaw is left alone.
pub fn align_behind(camera_yaw: f32, body_yaw: f32) -> f32 {
    let behind = body_yaw - camera_yaw;
    if f64::from(behind) > std::f64::consts::PI {
        (f64::from(camera_yaw) + std::f64::consts::TAU) as f32
    } else if f64::from(camera_yaw - body_yaw) > std::f64::consts::PI {
        (f64::from(camera_yaw) - std::f64::consts::TAU) as f32
    } else {
        camera_yaw
    }
}

/// The pitch the zoom asks for: 0.8 rad below the horizon under zoom 1000,
/// then 0.000125 rad steeper per unit. The sum is a double, as the
/// original's is.
pub fn pitch_floor(zoom: f32) -> f32 {
    if PITCH_ZOOM_BASE <= zoom {
        ((f64::from(zoom) - f64::from(PITCH_ZOOM_BASE)) * PITCH_PER_ZOOM + f64::from(PITCH_BASE))
            as f32
    } else {
        PITCH_BASE
    }
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

/// A free zoom after wheel `steps`: 0.2 % of the current value a step, but
/// never less than 0.5 % of the whole range, subtracted from the zoom and
/// held in `[min, max]`. The original scales the sign of a step that is
/// too small, so a zero step stays zero.
pub fn zoom_steps(zoom: f32, steps: f32, min: f32, max: f32) -> f32 {
    let mut delta = steps * zoom * ZOOM_RATE;
    let least = (max - min) * ZOOM_STEP_SHARE;
    if delta.abs() < least {
        delta = least * sign_or_zero(delta);
    }
    (zoom - delta).clamp(min, max)
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
    /// Radians below the horizon, the sign the controller stores.
    pub pitch: f32,
}

/// Places the camera at `at` (height-clamped) looking along `back`, with
/// the eye `distance` away when one is given; without one the distance is
/// how far `eye` lies from the point and the backward vector is that
/// direction normalised. The yaw is the engine's cheap atan2 of the
/// vector's x and z, and the pitch the engine's cheap arcsine.
pub fn place(at: Vec3, back: Vec3, distance: Option<f32>, limits: HeightLimits) -> Placement {
    let at = limits.apply(at);
    let (back, distance) = match distance {
        // The original takes a given direction as it is; a caller that
        // gives a distance gives a unit vector (guess: no normalisation
        // there).
        Some(distance) if distance >= DISTANCE_EPSILON => (back, distance),
        _ => {
            let vector = back - at;
            let distance = vector.length();
            if distance >= DISTANCE_EPSILON {
                (vector / distance, distance)
            } else {
                (Vec3::Z, 0.0)
            }
        }
    };
    Placement {
        at,
        back,
        distance,
        yaw: head_atan2_xz(back.x, back.z),
        pitch: -quick_asin(back.y),
    }
}

/// The distance after the fixed-distance camera takes its first value: at
/// least `min` (10 by default) and at most `max` (the zoom range's top).
pub fn clamp_distance(distance: f32, min: f32, max: f32) -> f32 {
    if distance < min {
        min
    } else if distance > max {
        max
    } else {
        distance
    }
}

/// Pulls a pitch into the limits the fixed-distance camera keeps: the
/// original stores `−pitch` and holds it between `−cap` and `−floor`.
pub fn clamp_fixed_pitch(pitch: f32, floor: f32, cap: f32) -> f32 {
    if floor > pitch {
        floor
    } else if pitch > cap {
        cap
    } else {
        pitch
    }
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
/// and the same divisor scales the preset's angles.
pub const PRESET_SCALE: f32 = 59.0 * 59.0;

/// A camera preset's raw fields, in the order the original's record keeps
/// them. Their roles were read from the fields the preset writes (guesses
/// where a field only feeds an editor): a zero asks for the default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresetFields {
    /// Flags word. `0x200` installs the wheel's stop easing, `0x200000`
    /// the fixed-distance camera, `0x100000` the editor's drag, `0x400000`
    /// the editor's target scan.
    pub flags: u32,
    /// Field of view scale (default 280).
    pub view: f32,
    /// Angle whose 59th root scales with the zoom (default 4719934.5).
    pub angle: f32,
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
    /// A free field the original copies to the controller (0xD; its role
    /// was not established).
    pub extra: f32,
}

impl Default for PresetFields {
    fn default() -> Self {
        Self {
            flags: 0,
            view: 0.0,
            angle: 0.0,
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
    pub angle: f32,
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
    /// The pitch speed and the zoom speed the preset sets (fields 0x17C and
    /// 0x154 in the original).
    pub pitch_speed: f32,
    pub zoom_speed: f32,
}

/// The field of view scale a preset value gives: `value / 59²`.
pub fn fov_scale(value: f32) -> f32 {
    value / PRESET_SCALE
}

/// `exp(log2(value) / 59)`: the preset's 59th-root scaling, as the original
/// does it with the runtime library's log2 and exp in double precision.
fn root59(value: f32) -> f32 {
    (f64::from(value).log2() / 59.0).exp2() as f32
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
    let fixed = fields.flags & 0x20_0000 != 0;
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
        root59(fields.rate)
    } else {
        1.0
    };
    let pitch_speed = if fixed {
        5.0
    } else {
        max_distance * 15.0 / PRESET_SCALE
    };
    let zoom_speed = if fixed {
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
        angle: root59(if fields.angle == 0.0 {
            4719934.5
        } else {
            fields.angle
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

/// The segment the camera's cut-away detector casts: it starts two camera
/// distances behind the looked-at point, along the backward vector, and
/// runs back along it for `2·distance − 1 m`, stopping a metre short of the
/// point. Returns the start, the (unit) direction and the length.
pub fn cut_away_segment(focus: Vec3, back: Vec3, distance: f32) -> (Vec3, Vec3, f32) {
    let start = focus - back * (2.0 * distance);
    (start, back, 2.0 * distance - 1.0)
}

/// The offset one kick puts on the eye, before the chase camera's shake
/// carries it: a small random offset plus the camera's right axis, jittered
/// vertically, normalised to `size`. The three random draws are the
/// original's order; two more are taken only when the right axis is
/// degenerate.
///
/// [guess: the original draws from the engine's global generator; Exarust
/// passes its own so a replay keeps its stream]
pub fn kick_offset(right: Vec3, size: f32, draw: &mut impl FnMut() -> f32) -> Vec3 {
    let first = draw() - 0.5;
    let second = (f64::from(draw()) * 0.15) as f32;
    let third = draw() - 0.5;
    let mut offset = Vec3::new(first, second, third);
    let mut basis = right * sign_or_zero(first);
    if basis.length_squared() < 0.1 {
        basis = right * sign_or_zero(draw() - 0.5);
        basis.y += (draw() - 0.5) * 4.0;
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
    use std::f32::consts::{PI, TAU};

    #[test]
    fn a_camera_yaw_stays_within_half_a_turn_of_the_body() {
        // Already behind the body: unchanged, also just inside the turn.
        assert_eq!(align_behind(0.0, 0.0), 0.0);
        assert_eq!(align_behind(0.5, 0.0), 0.5);
        assert_eq!(align_behind(-0.5, 0.0), -0.5);
        // More than π behind: one turn forward, within the half turn.
        let wrapped = align_behind(-3.0, 3.0);
        assert!((wrapped - (-3.0 + TAU)).abs() < 1e-6, "{wrapped}");
        // More than π ahead: one turn back.
        let wrapped = align_behind(3.0, -3.0);
        assert!((wrapped - (3.0 - TAU)).abs() < 1e-6, "{wrapped}");
        // The f32 π itself is already a hair more than the f64 π, so an
        // exact ±π pair wraps by a whole turn (the original's compare is in
        // f64 over the f32 difference).
        let wrapped = align_behind(-PI, 0.0);
        assert!((wrapped - PI).abs() < 1e-6, "{wrapped}");
        let wrapped = align_behind(PI, 0.0);
        assert!((wrapped + PI).abs() < 1e-6, "{wrapped}");
        // A NaN takes neither branch.
        assert!(align_behind(f32::NAN, 0.0).is_nan());
    }

    #[test]
    fn the_pitch_floor_rises_with_the_zoom() {
        assert_eq!(pitch_floor(0.0), 0.8);
        assert_eq!(pitch_floor(999.0), 0.8);
        assert_eq!(pitch_floor(1000.0), 0.8);
        assert!((pitch_floor(2000.0) - 0.925).abs() < 1e-6);
        assert!((pitch_floor(1300.0) - 0.8375).abs() < 1e-6);
        // The original adds in double precision.
        assert_eq!(pitch_floor(1000.5), 0.8 + 0.5 * 0.000125);
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
    fn a_free_zoom_steps_by_a_fifth_of_a_percent_or_its_least_step() {
        // 0.2 % of 1000 is 2, below the 6 of a 1200-wide range.
        assert_eq!(zoom_steps(1000.0, 1.0, 800.0, 2000.0), 994.0);
        assert_eq!(zoom_steps(1000.0, -1.0, 800.0, 2000.0), 1006.0);
        // A zero step stays zero.
        assert_eq!(zoom_steps(1000.0, 0.0, 800.0, 2000.0), 1000.0);
        // A large value's 0.2 % is the step: 2000 × 0.002 = 4 < 6, too.
        assert_eq!(zoom_steps(2000.0, 1.0, 800.0, 2000.0), 1994.0);
        // A wide range still takes 0.5 % of it.
        assert_eq!(zoom_steps(5000.0, 1.0, 10.0, 10000.0), 4950.05);
        // The result stays in the range.
        assert_eq!(zoom_steps(800.0, 1.0, 800.0, 2000.0), 800.0);
        assert_eq!(zoom_steps(2000.0, -1.0, 800.0, 2000.0), 2000.0);
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
        assert!((placed.distance - Vec3::new(0.0, 10.0, 5.0).length()).abs() < 1e-5);
        // Straight behind: yaw 0, pitch negative (the eye above).
        assert!(placed.back.z > 0.0);
        assert!(placed.pitch < 0.0);
        assert_eq!(placed.yaw, 0.0);
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
        // Behind +X: the engine's atan2(x, z) gives +π/2.
        let placed = place(
            Vec3::ZERO,
            Vec3::X,
            None,
            HeightLimits {
                min: -1e9,
                max: 1e9,
            },
        );
        assert!((placed.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
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
        assert!(
            (preset.angle - 1.297_530_5).abs() < 1e-6,
            "{}",
            preset.angle
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
        // The fixed-distance camera swaps in its own speeds.
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
            angle: 8.0,
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
        assert!((preset.angle - 8f32.powf(1.0 / 59.0)).abs() < 1e-5);
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
    fn the_cut_away_segment_starts_behind_the_eye_and_stops_short() {
        let (start, direction, length) =
            cut_away_segment(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, 0.0, 1.0), 20.0);
        assert_eq!(start, Vec3::new(0.0, 10.0, -40.0));
        assert_eq!(direction, Vec3::Z);
        assert_eq!(length, 39.0);
        // A small distance can make a negative length; the detector's
        // segment then runs the other way, as the original's does.
        let (_, _, length) = cut_away_segment(Vec3::ZERO, Vec3::Z, 0.4);
        assert!(length < 0.0);
    }

    #[test]
    fn a_kick_is_a_normalised_random_offset_from_the_camera_right() {
        // Draws: 0.75 (first), 0.5 (second), 0.5 (third).
        let mut draws = [0.75, 0.5, 0.5].into_iter();
        let offset = kick_offset(Vec3::X, 2.0, &mut || draws.next().unwrap());
        // The right axis (+1) plus (0.25, 0.075, 0): normalised to 2 m.
        let want = Vec3::new(1.25, 0.075, 0.0).normalize() * 2.0;
        assert!((offset - want).length() < 1e-6, "{offset:?} {want:?}");
        // The first draw's sign flips the basis: −X plus the offset.
        let mut draws = [0.25, 0.5, 0.5].into_iter();
        let offset = kick_offset(Vec3::X, 2.0, &mut || draws.next().unwrap());
        let want = Vec3::new(-1.25, 0.075, 0.0).normalize() * 2.0;
        assert!((offset - want).length() < 1e-6, "{offset:?} {want:?}");
    }
}
