//! The samplers' heights: what the pose-only sampler (the placement's) and
//! the living sampler compute after posing every part, from the clip root
//! and the foot contacts. Lengths are in cm, as the original's.
//!
//! * The **stand height** is the clip root's height over the lowest foot
//!   contact of the posed (twisted) parts, less the sole height; the
//!   **leg reach** is the root's height less half the sum of the contacts'
//!   heights under the clip pose. A hovering body (the race's hover flag
//!   for the placement, the body's no-foot-probe byte for the living
//!   sampler) takes 10 cm under the root for both.
//! * The **pelvis target** is the stand height plus a margin: on the
//!   placement it is set outright, in the living sampler it is eased 0.1
//!   of the way a tick (the walking condition's ease grows with the run
//!   blend). Both end multiplied by the body's pelvis-height factor (1
//!   from the constructor; nothing in the character class changes it).
//!
//! Exarust's carry computes its own pelvis target from the feet
//! ([`super::lift`]); these are the samplers' values, tested against the
//! original's instructions but not read by the carry.

/// The lowest contact starts this high (cm), so a body without contacts
/// stands at 0.
const NO_CONTACT: f32 = 5000.0;
/// A hovering body's heights sit this far under its root (cm).
const HOVER_DROP: f32 = 10.0;
/// The margin over the stand height when not stepping (cm).
const NOT_STEPPING: f32 = 5.0;
/// Crouching past this (double) shortens the target...
const CROUCHED: f64 = 0.95;
/// ...to this share of it while stepping (double)...
const CROUCHED_STEPPING: f64 = 0.8;
/// ...and to this one otherwise (double).
const CROUCHED_STANDING: f64 = 0.7;
/// The living sampler's ease toward the target (double).
const EASE: f64 = 0.1;
/// A get-up resets the living target while its progress is under this
/// (double).
const EARLY_RISE: f64 = 0.2;

/// One foot contact as the samplers read it (cm, body-root space): its
/// height under the clip pose and under the posed parts (after the torso
/// twist, the femur posture and the hunch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactHeight {
    pub clip: f32,
    pub posed: f32,
}

/// The stand height and the leg reach (cm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandHeights {
    pub stand: f32,
    pub reach: f32,
}

/// The samplers' stand height and leg reach from the clip root's height
/// `root` (cm), the foot `contacts` and the feet's `sole` height.
///
/// The lowest contact starts at 5000 and takes each posed height less the
/// sole that is under it (a NaN never is). The stand height is the root
/// less that lowest unless the lowest is at or over the root (then 0; a
/// NaN root subtracts). The reach is the root less half the clip heights'
/// sum, summed from 0 in contact order. Hovering, both are the root less
/// 10.
pub fn stand_heights(
    root: f32,
    contacts: &[ContactHeight],
    sole: f32,
    hovering: bool,
) -> StandHeights {
    if hovering {
        let h = root - HOVER_DROP;
        return StandHeights { stand: h, reach: h };
    }
    let mut lowest = NO_CONTACT;
    let mut sum = 0.0f32;
    for c in contacts {
        sum += c.clip;
        let v = c.posed - sole;
        if lowest > v {
            lowest = v;
        }
    }
    let stand = if lowest >= root { 0.0 } else { root - lowest };
    StandHeights {
        stand,
        reach: root - sum * 0.5,
    }
}

/// What the pelvis target reads of the body besides its heights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PelvisAim {
    /// The condition is the plain walking one (1).
    pub walking: bool,
    /// The condition has the off-feet bit (0x200; lying and getting up).
    pub off_feet: bool,
    /// The condition is getting up (0x300).
    pub getting_up: bool,
    /// The clip playing is Unconscious 1 (action 29), whose stand height
    /// does not count.
    pub unconscious: bool,
    /// The run blend.
    pub run: f32,
    /// The body scale.
    pub scale: f32,
    /// A step is under way.
    pub stepping: bool,
    /// The crouch amount.
    pub crouch: f32,
    /// The clip's progress (the living sampler's get-up reset reads it).
    pub progress: f32,
    /// The pelvis-height factor (1).
    pub factor: f32,
}

/// The margin shared by both samplers off the walking condition: the stand
/// height (0 under the Unconscious clip) plus twice the scale, plus 5 when
/// not stepping, in single precision in that order.
fn margin(stand: f32, aim: &PelvisAim) -> f32 {
    let mut s = stand + 2.0 * aim.scale;
    if !aim.stepping {
        s += NOT_STEPPING;
    }
    s
}

/// Crouched past 0.95 (double; a NaN is not) and on its feet, the target
/// is the margin times 0.8 stepping or 0.7 standing, in double.
fn crouched(s: f32, aim: &PelvisAim) -> Option<f32> {
    if f64::from(aim.crouch) > CROUCHED && !aim.off_feet {
        let k = if aim.stepping {
            CROUCHED_STEPPING
        } else {
            CROUCHED_STANDING
        };
        Some((f64::from(s) * k) as f32)
    } else {
        None
    }
}

/// The pose-only sampler's pelvis target (cm), set outright: walking, it
/// is (3 − 3·run) + (stand + the pelvis's posed height `pelvis`), single
/// precision in that order (the original adds the pelvis's own placement
/// height, not twice the run blend as the living sampler does); otherwise
/// the margin, shortened when crouched. Times the factor.
pub fn placement_pelvis_target(stand: f32, pelvis: f32, aim: &PelvisAim) -> f32 {
    let target = if aim.walking {
        (3.0 - 3.0 * aim.run) + (stand + pelvis)
    } else {
        let stand = if aim.unconscious { 0.0 } else { stand };
        let s = margin(stand, aim);
        crouched(s, aim).unwrap_or(s)
    };
    target * aim.factor
}

/// The living sampler's pelvis target (cm) from last tick's `previous`.
/// Hovering (`hover_root` the clip root's height), it is the root less 10.
/// Walking, s = stand + (2·run + 3) in single precision and the target is
/// (0.1 + 0.1·run)·s + (0.9 − 0.1·run)·previous in double. Otherwise the
/// stand height counts for nothing under the Unconscious clip, and a
/// get-up whose progress is under 0.2 (double; a NaN is not) clears both
/// it and the previous target; the margin, shortened when crouched, is set
/// outright, else eased: 0.1·margin + 0.9·previous in double. Times the
/// factor.
pub fn living_pelvis_target(
    previous: f32,
    stand: f32,
    hover_root: Option<f32>,
    aim: &PelvisAim,
) -> f32 {
    if let Some(root) = hover_root {
        return (root - HOVER_DROP) * aim.factor;
    }
    let target = if aim.walking {
        let s = stand + (2.0 * aim.run + 3.0);
        let r = f64::from(aim.run) * EASE;
        ((EASE + r) * f64::from(s) + (0.9 - r) * f64::from(previous)) as f32
    } else {
        let (mut previous, mut stand) = (previous, stand);
        if aim.unconscious {
            stand = 0.0;
        } else if aim.getting_up && f64::from(aim.progress) < EARLY_RISE {
            previous = 0.0;
            stand = 0.0;
        }
        let s = margin(stand, aim);
        match crouched(s, aim) {
            Some(c) => c,
            None => (f64::from(s) * EASE + f64::from(previous) * 0.9) as f32,
        }
    };
    target * aim.factor
}

#[cfg(test)]
#[path = "body_pose_heights_test.rs"]
mod tests;
