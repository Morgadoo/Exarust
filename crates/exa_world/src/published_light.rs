//! The light frame the scene environment publishes to the engine's scene
//! light records: the sun's direction and its derived values, the reach the
//! sun is placed at, and the sky light's direction.
//!
//! The original keeps these in four module records that the renderer and the
//! skinned-mesh pass read. The environment's per-frame apply copies its own
//! sun and sky records into them. This model keeps the same values with the
//! original's arithmetic so the running renderer can take them from one
//! place:
//!
//! - [`PublishedLight::set_sun`] stores the sun's four-float record, derives
//!   the record's direction times its fourth lane and the fourth lane's
//!   square root, then re-publishes the reach it held (through the zeroed
//!   reach, so a reach under the dead band does not rescale anything).
//! - [`PublishedLight::set_reach`] stores a reach only when it differs from
//!   the published one by at least 0.01 (compared in double precision after
//!   the difference is taken and made positive in single precision), and
//!   scales the sun record's direction by it — halved while the original's
//!   mode flag is clear.
//! - [`PublishedLight::set_sky`] stores the sky light's four-float record
//!   verbatim.
//!
//! The records' identity beyond "the sun" and "the sky light" is inferred:
//! the fourth lane of the sun record is used as an intensity, and it is
//! published both scaled into the direction and as its square root, so the
//! original evidently pre-multiplies a strength into the direction. That
//! reading is a guess, as is the mode flag's meaning (only its halving is
//! established). The reader of the sky record outside this unit takes a dot
//! product with a view direction, so it is a direction there.
//!
//! Evidence: the unit's setters and the environment apply that calls them;
//! the dead band and the scaling are from the setters' instructions.

// `!(a < b)` is deliberate: it is the original's branch with a NaN (a NaN
// difference publishes the reach).
#![allow(clippy::neg_cmp_op_on_partial_ord)]

/// The reach's dead band: a new reach within this of the published one is
/// ignored. The difference is taken in single precision, made positive, and
/// compared in double precision, as the original does.
pub const REACH_DEAD_BAND: f64 = 0.01;

/// The scene light records the environment publishes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PublishedLight {
    /// The sun's record: a direction in x, y and z with the strength the
    /// published values are derived from in w.
    pub sun: [f32; 4],
    /// The sun's direction times the record's fourth lane.
    pub sun_scaled: [f32; 3],
    /// The square root of the record's fourth lane.
    pub sun_root: f32,
    /// The published reach: the distance the sun is placed at. Zero until
    /// a reach passes the dead band.
    pub reach: f32,
    /// The sun record's direction times the published reach (halved while
    /// the mode flag is clear).
    pub reach_vector: [f32; 3],
    /// The sky light's record, stored verbatim: a direction in x, y and z.
    pub sky: [f32; 4],
    /// The original's one-byte mode flag. Its readers are outside this
    /// unit, so its meaning is not established; a clear flag halves the
    /// reach (a guess at what it selects: the sun at half its distance).
    pub mode: bool,
}

impl Default for PublishedLight {
    fn default() -> Self {
        Self {
            sun: [0.0; 4],
            sun_scaled: [0.0; 3],
            sun_root: 0.0,
            reach: 0.0,
            reach_vector: [0.0; 3],
            sky: [0.0; 4],
            mode: false,
        }
    }
}

impl PublishedLight {
    /// Sets the sun record and derives its values, then re-publishes the
    /// reach held. The reach is first zeroed the way the original's setter
    /// does, so the re-publish always passes the dead band for a reach of
    /// 0.01 or more; a smaller reach leaves the published vector as it was
    /// and the reach at zero.
    pub fn set_sun(&mut self, record: [f32; 4]) {
        self.sun = record;
        let w = record[3];
        self.sun_scaled = [record[0] * w, record[1] * w, record[2] * w];
        self.sun_root = w.sqrt();
        let reach = self.reach;
        self.reach = 0.0;
        self.set_reach(reach);
    }

    /// Stores a reach when it differs from the published one by at least
    /// [`REACH_DEAD_BAND`], and scales the sun record's direction by it. A
    /// difference that is not less than the band counts — a NaN reach is
    /// published. The stored value is the argument; the scaling factor is
    /// the argument halved while [`PublishedLight::mode`] is clear.
    pub fn set_reach(&mut self, reach: f32) {
        let difference = (self.reach - reach).abs();
        if !(f64::from(difference) < REACH_DEAD_BAND) {
            self.reach = reach;
            let scale = if self.mode { reach } else { reach * 0.5 };
            self.reach_vector = [
                self.sun[0] * scale,
                self.sun[1] * scale,
                self.sun[2] * scale,
            ];
        }
    }

    /// Stores the sky light's record verbatim.
    pub fn set_sky(&mut self, record: [f32; 4]) {
        self.sky = record;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_record_is_scaled_and_rooted() {
        let mut light = PublishedLight::default();
        light.set_sun([0.6, 0.0, -0.8, 2.0]);
        assert_eq!(light.sun, [0.6, 0.0, -0.8, 2.0]);
        assert_eq!(light.sun_scaled, [1.2, 0.0, -1.6]);
        assert_eq!(light.sun_root, 2.0f32.sqrt());
        // The held reach was zero, so nothing is published past the band.
        assert_eq!(light.reach, 0.0);
        assert_eq!(light.reach_vector, [0.0; 3]);
    }

    #[test]
    fn a_reach_scales_the_direction_by_half_while_the_mode_is_clear() {
        let mut light = PublishedLight::default();
        light.set_sun([0.0, 1.0, 0.0, 1.0]);
        light.set_reach(10.0);
        assert_eq!(light.reach, 10.0);
        assert_eq!(light.reach_vector, [0.0, 5.0, 0.0]);
        light.mode = true;
        light.set_reach(20.0);
        assert_eq!(light.reach_vector, [0.0, 20.0, 0.0]);
    }

    #[test]
    fn a_reach_inside_the_dead_band_leaves_the_vector_alone() {
        let mut light = PublishedLight::default();
        light.set_sun([0.0, 1.0, 0.0, 1.0]);
        light.set_reach(10.0);
        // A new sun does not rescale while the reach is inside the band.
        light.sun = [1.0, 0.0, 0.0, 1.0];
        light.set_reach(10.005);
        assert_eq!(light.reach, 10.0);
        assert_eq!(light.reach_vector, [0.0, 5.0, 0.0]);
        // 0.02 past the published reach passes.
        light.set_reach(10.02);
        assert_eq!(light.reach, 10.02);
        assert_eq!(light.reach_vector, [5.01, 0.0, 0.0]);
    }

    #[test]
    fn a_nan_reach_is_published() {
        let mut light = PublishedLight::default();
        light.set_sun([0.0, 1.0, 0.0, 1.0]);
        light.set_reach(f32::NAN);
        assert!(light.reach.is_nan());
        assert!(light.reach_vector[1].is_nan());
    }

    #[test]
    fn setting_a_sun_republishes_the_reach_through_zero() {
        let mut light = PublishedLight::default();
        light.set_sun([0.0, 1.0, 0.0, 1.0]);
        light.set_reach(10.0);
        // The new sun re-publishes 10: the reach is zeroed first, so the
        // band passes and the new direction is scaled.
        light.set_sun([1.0, 0.0, 0.0, 1.0]);
        assert_eq!(light.reach, 10.0);
        assert_eq!(light.reach_vector, [5.0, 0.0, 0.0]);
        // A held reach under the band is not re-published: the zeroed reach
        // stays and the vector keeps the last direction.
        let mut small = PublishedLight::default();
        small.set_sun([0.0, 1.0, 0.0, 1.0]);
        small.set_reach(0.02);
        small.set_reach(0.005);
        assert_eq!(small.reach, 0.005);
        assert_eq!(small.reach_vector, [0.0, 0.0025, 0.0]);
        small.set_sun([1.0, 0.0, 0.0, 1.0]);
        assert_eq!(small.reach, 0.0);
        assert_eq!(small.reach_vector, [0.0, 0.0025, 0.0]);
    }

    #[test]
    fn the_sky_record_is_stored_verbatim() {
        let mut light = PublishedLight::default();
        light.set_sky([0.25, -0.5, 0.75, 0.0]);
        assert_eq!(light.sky, [0.25, -0.5, 0.75, 0.0]);
    }
}
